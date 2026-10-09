/**
 * src/chain/client.ts
 *
 * Soroban RPC client wrapper for the land-registry contract.
 *
 * This module wraps every public contract function with a typed TypeScript
 * function that builds, signs, and submits the corresponding Soroban
 * transaction. It is the single integration point between the backend and
 * the on-chain contract.
 *
 * Signing strategy (per README "Trust & security model"):
 *   - In production, citizens and registrars sign their own transactions
 *     client-side via Freighter. The backend does NOT custody their keys.
 *   - For server-side operations (e.g. the smoke test, a batch registrar
 *     flow), a BACKEND_SECRET_KEY env var drives a local keypair signer.
 *   - The `signer` parameter is intentionally pluggable so the same functions
 *     work in both contexts.
 *
 * NOTE: This client is built against the stellar-sdk directly because the
 * `stellar contract bindings typescript` command requires a live deployed
 * contract and a running network. The generated bindings (once available)
 * can replace the manual XDR construction below with typed wrappers.
 * See README "Development workflow" step 2.
 */

import {
  Keypair,
  Networks,
  SorobanRpc,
  TransactionBuilder,
  BASE_FEE,
  Contract,
  xdr,
  nativeToScVal,
  Address,
  scValToNative,
} from "@stellar/stellar-sdk";

// ── Types mirroring the on-chain data model (README "Data model") ──────────

export interface TitleRecord {
  id: string;         // hex-encoded BytesN<32>
  docHash: string;    // hex-encoded BytesN<32>
  storageRef: string;
  gpsLat: bigint;
  gpsLng: bigint;
  owner: string;      // Stellar address G...
  status: TitleStatus;
  createdAt: bigint;
  updatedAt: bigint;
}

export type TitleStatus = "Active" | "PendingTransfer" | "Disputed" | "Revoked";

export interface TransferProposal {
  titleId: string;
  newOwner: string;
  proposer: string;
  approvals: string[];
  threshold: number;
  expiresAt: bigint;
}

export interface RegisterTitleArgs {
  registrar: Keypair;
  id: Uint8Array;          // 32 bytes
  docHash: Uint8Array;     // 32 bytes
  storageRef: string;
  gpsLat: bigint;
  gpsLng: bigint;
  owner: string;           // Stellar G... address
}

// ── Client configuration ───────────────────────────────────────────────────

export interface LandRegistryClientConfig {
  rpcUrl: string;
  networkPassphrase: string;
  contractId: string;
}

function defaultConfig(): LandRegistryClientConfig {
  const rpcUrl = process.env.SOROBAN_RPC_URL;
  const networkPassphrase = process.env.STELLAR_NETWORK_PASSPHRASE;
  const contractId = process.env.LAND_REGISTRY_CONTRACT_ID;

  if (!rpcUrl || !networkPassphrase || !contractId) {
    throw new Error(
      "Missing required env vars: SOROBAN_RPC_URL, STELLAR_NETWORK_PASSPHRASE, LAND_REGISTRY_CONTRACT_ID"
    );
  }
  return { rpcUrl, networkPassphrase, contractId };
}

// ── Helper: submit a transaction and wait for confirmation ─────────────────

async function submitAndWait(
  server: SorobanRpc.Server,
  tx: string
): Promise<SorobanRpc.Api.GetTransactionResponse> {
  const sendResponse = await server.sendTransaction(tx);
  if (sendResponse.status === "ERROR") {
    throw new Error(`Transaction send failed: ${JSON.stringify(sendResponse)}`);
  }

  const hash = sendResponse.hash;
  let response: SorobanRpc.Api.GetTransactionResponse;
  let attempts = 0;
  const maxAttempts = 20;

  do {
    await new Promise((r) => setTimeout(r, 1000));
    response = await server.getTransaction(hash);
    attempts++;
  } while (
    response.status === SorobanRpc.Api.GetTransactionStatus.NOT_FOUND &&
    attempts < maxAttempts
  );

  if (response.status !== SorobanRpc.Api.GetTransactionStatus.SUCCESS) {
    throw new Error(
      `Transaction failed or timed out: ${JSON.stringify(response)}`
    );
  }

  return response;
}

// ── Helper: build a base transaction ──────────────────────────────────────

async function buildTx(
  server: SorobanRpc.Server,
  sourceKeypair: Keypair,
  networkPassphrase: string,
  contractId: string,
  method: string,
  args: xdr.ScVal[]
): Promise<string> {
  const sourceAccount = await server.getAccount(sourceKeypair.publicKey());
  const contract = new Contract(contractId);

  const tx = new TransactionBuilder(sourceAccount, {
    fee: BASE_FEE,
    networkPassphrase,
  })
    .addOperation(contract.call(method, ...args))
    .setTimeout(30)
    .build();

  const preparedTx = await server.prepareTransaction(tx);
  preparedTx.sign(sourceKeypair);
  return preparedTx.toXDR();
}

// ── Public client functions ────────────────────────────────────────────────

export class LandRegistryClient {
  private readonly server: SorobanRpc.Server;
  private readonly cfg: LandRegistryClientConfig;

  constructor(cfg?: LandRegistryClientConfig) {
    this.cfg = cfg ?? defaultConfig();
    this.server = new SorobanRpc.Server(this.cfg.rpcUrl, {
      allowHttp: this.cfg.rpcUrl.startsWith("http://"),
    });
  }

  /**
   * Register a new title on-chain.
   * The registrar keypair signs the transaction.
   * Per README: only hashes and GPS go on-chain, never document content.
   */
  async registerTitle(args: RegisterTitleArgs): Promise<TitleRecord> {
    const params = xdr.ScVal.scvMap([
      new xdr.ScMapEntry({
        key: xdr.ScVal.scvSymbol("id"),
        val: nativeToScVal(Buffer.from(args.id), { type: "bytes" }),
      }),
      new xdr.ScMapEntry({
        key: xdr.ScVal.scvSymbol("doc_hash"),
        val: nativeToScVal(Buffer.from(args.docHash), { type: "bytes" }),
      }),
      new xdr.ScMapEntry({
        key: xdr.ScVal.scvSymbol("gps_lat"),
        val: nativeToScVal(args.gpsLat, { type: "i64" }),
      }),
      new xdr.ScMapEntry({
        key: xdr.ScVal.scvSymbol("gps_lng"),
        val: nativeToScVal(args.gpsLng, { type: "i64" }),
      }),
      new xdr.ScMapEntry({
        key: xdr.ScVal.scvSymbol("owner"),
        val: new Address(args.owner).toScVal(),
      }),
      new xdr.ScMapEntry({
        key: xdr.ScVal.scvSymbol("storage_ref"),
        val: nativeToScVal(args.storageRef, { type: "string" }),
      }),
    ]);

    const xdrTx = await buildTx(
      this.server,
      args.registrar,
      this.cfg.networkPassphrase,
      this.cfg.contractId,
      "register_title",
      [new Address(args.registrar.publicKey()).toScVal(), params]
    );

    const result = await submitAndWait(this.server, xdrTx);
    return scValToNative(result.returnValue!) as TitleRecord;
  }

  /** Read a title record directly from the chain (never from a cache). */
  async getTitle(titleId: Uint8Array): Promise<TitleRecord> {
    const result = await this.server.simulateTransaction(
      await this._buildReadTx("get_title", [
        nativeToScVal(Buffer.from(titleId), { type: "bytes" }),
      ])
    );

    if (SorobanRpc.Api.isSimulationError(result)) {
      throw new Error(`get_title simulation failed: ${result.error}`);
    }

    const simResult = result as SorobanRpc.Api.SimulateTransactionSuccessResponse;
    return scValToNative(simResult.result!.retval) as TitleRecord;
  }

  /**
   * Verify a document hash directly against the chain.
   *
   * Per README "Development workflow": "verification actions always recompute
   * the hash client-side and check it against a direct RPC read of get_title."
   * The frontend calls this; the indexer-backed REST API is never used for
   * security-critical verification.
   */
  async verifyHash(
    titleId: Uint8Array,
    candidateHash: Uint8Array
  ): Promise<boolean> {
    const result = await this.server.simulateTransaction(
      await this._buildReadTx("verify_hash", [
        nativeToScVal(Buffer.from(titleId), { type: "bytes" }),
        nativeToScVal(Buffer.from(candidateHash), { type: "bytes" }),
      ])
    );

    if (SorobanRpc.Api.isSimulationError(result)) {
      throw new Error(`verify_hash simulation failed: ${result.error}`);
    }

    const simResult = result as SorobanRpc.Api.SimulateTransactionSuccessResponse;
    return scValToNative(simResult.result!.retval) as boolean;
  }

  /** Initiate a transfer proposal. Signed by the current owner. */
  async initiateTransfer(
    ownerKeypair: Keypair,
    titleId: Uint8Array,
    newOwner: string
  ): Promise<TransferProposal> {
    const xdrTx = await buildTx(
      this.server,
      ownerKeypair,
      this.cfg.networkPassphrase,
      this.cfg.contractId,
      "initiate_transfer",
      [
        new Address(ownerKeypair.publicKey()).toScVal(),
        nativeToScVal(Buffer.from(titleId), { type: "bytes" }),
        new Address(newOwner).toScVal(),
      ]
    );
    const result = await submitAndWait(this.server, xdrTx);
    return scValToNative(result.returnValue!) as TransferProposal;
  }

  /** Co-sign a pending transfer. Signed by a registrar. */
  async coSignTransfer(
    registrarKeypair: Keypair,
    titleId: Uint8Array
  ): Promise<void> {
    const xdrTx = await buildTx(
      this.server,
      registrarKeypair,
      this.cfg.networkPassphrase,
      this.cfg.contractId,
      "co_sign_transfer",
      [
        new Address(registrarKeypair.publicKey()).toScVal(),
        nativeToScVal(Buffer.from(titleId), { type: "bytes" }),
      ]
    );
    await submitAndWait(this.server, xdrTx);
  }

  /** Explicitly execute a transfer that has already met the threshold. */
  async executeTransfer(
    callerKeypair: Keypair,
    titleId: Uint8Array
  ): Promise<void> {
    const xdrTx = await buildTx(
      this.server,
      callerKeypair,
      this.cfg.networkPassphrase,
      this.cfg.contractId,
      "execute_transfer",
      [nativeToScVal(Buffer.from(titleId), { type: "bytes" })]
    );
    await submitAndWait(this.server, xdrTx);
  }

  /** Cancel a pending transfer. Signed by the proposer or admin. */
  async cancelTransfer(
    callerKeypair: Keypair,
    titleId: Uint8Array
  ): Promise<void> {
    const xdrTx = await buildTx(
      this.server,
      callerKeypair,
      this.cfg.networkPassphrase,
      this.cfg.contractId,
      "cancel_transfer",
      [
        new Address(callerKeypair.publicKey()).toScVal(),
        nativeToScVal(Buffer.from(titleId), { type: "bytes" }),
      ]
    );
    await submitAndWait(this.server, xdrTx);
  }

  /** Flag a title as disputed. Signed by a registrar. */
  async flagDispute(
    registrarKeypair: Keypair,
    titleId: Uint8Array
  ): Promise<void> {
    const xdrTx = await buildTx(
      this.server,
      registrarKeypair,
      this.cfg.networkPassphrase,
      this.cfg.contractId,
      "flag_dispute",
      [
        new Address(registrarKeypair.publicKey()).toScVal(),
        nativeToScVal(Buffer.from(titleId), { type: "bytes" }),
      ]
    );
    await submitAndWait(this.server, xdrTx);
  }

  /** Resolve a disputed title. Signed by a registrar. */
  async resolveDispute(
    registrarKeypair: Keypair,
    titleId: Uint8Array,
    newStatus: TitleStatus
  ): Promise<void> {
    const statusScVal = xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(newStatus)]);
    const xdrTx = await buildTx(
      this.server,
      registrarKeypair,
      this.cfg.networkPassphrase,
      this.cfg.contractId,
      "resolve_dispute",
      [
        new Address(registrarKeypair.publicKey()).toScVal(),
        nativeToScVal(Buffer.from(titleId), { type: "bytes" }),
        statusScVal,
      ]
    );
    await submitAndWait(this.server, xdrTx);
  }

  // ── Private helpers ──────────────────────────────────────────────────────

  /**
   * Build a read-only (simulation-only) transaction for view functions.
   * These don't need signing; we just need a dummy source account.
   */
  private async _buildReadTx(
    method: string,
    args: xdr.ScVal[]
  ): Promise<Parameters<SorobanRpc.Server["simulateTransaction"]>[0]> {
    // For simulations we need any valid funded account as the source.
    // The backend service account key covers this in dev; in production a
    // dedicated read-only horizon account can be used.
    const secretKey = process.env.BACKEND_SECRET_KEY;
    if (!secretKey) {
      throw new Error("BACKEND_SECRET_KEY is required for read simulations");
    }
    const kp = Keypair.fromSecret(secretKey);
    const sourceAccount = await this.server.getAccount(kp.publicKey());
    const contract = new Contract(this.cfg.contractId);

    return new TransactionBuilder(sourceAccount, {
      fee: BASE_FEE,
      networkPassphrase: this.cfg.networkPassphrase,
    })
      .addOperation(contract.call(method, ...args))
      .setTimeout(30)
      .build();
  }
}

/** Singleton factory — returns a client configured from environment variables. */
export function createClient(cfg?: LandRegistryClientConfig): LandRegistryClient {
  return new LandRegistryClient(cfg);
}
