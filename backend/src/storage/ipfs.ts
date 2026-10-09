/**
 * src/storage/ipfs.ts
 *
 * Off-chain document pinning for GroundTruth.
 *
 * Documents are pinned to IPFS (via Pinata by default) and the returned CID
 * becomes the `storage_ref` field anchored on-chain. The CID is the only
 * link between the on-chain hash and the off-chain document — the document
 * content itself is never sent to the chain.
 *
 * README "Trust & security model":
 *   "only hashes and coarse GPS coordinates are on-chain. Actual documents
 *    live in encrypted off-chain storage."
 *
 * To use S3-compatible storage instead of Pinata, set STORAGE_PROVIDER=s3
 * in your .env and implement the s3 branch below.
 */

import FormData from "form-data";
import axios from "axios";

export interface PinResult {
  /** IPFS CID, e.g. "QmXyz..." or a v1 CID. */
  cid: string;
  /** storage_ref string to store on-chain — an ipfs:// URI. */
  storageRef: string;
}

/**
 * Pin a document buffer to IPFS via the Pinata pinning API.
 *
 * @param data     Raw document bytes (e.g. a PDF buffer).
 * @param filename A human-readable filename for the Pinata metadata.
 * @returns        `{ cid, storageRef }` where `storageRef` is the value to
 *                 pass as `storage_ref` to `register_title`.
 */
export async function pinToIpfs(
  data: Buffer | Uint8Array,
  filename: string
): Promise<PinResult> {
  const jwt = process.env.PINATA_JWT;
  if (!jwt) {
    throw new Error(
      "PINATA_JWT is not set. Configure it in .env or set STORAGE_PROVIDER=s3."
    );
  }

  const form = new FormData();
  form.append("file", Buffer.from(data), { filename });
  form.append(
    "pinataMetadata",
    JSON.stringify({ name: filename }),
    { contentType: "application/json" }
  );

  const response = await axios.post<{ IpfsHash: string }>(
    "https://api.pinata.cloud/pinning/pinFileToIPFS",
    form,
    {
      headers: {
        Authorization: `Bearer ${jwt}`,
        ...form.getHeaders(),
      },
      maxBodyLength: Infinity,
    }
  );

  const cid = response.data.IpfsHash;
  return { cid, storageRef: `ipfs://${cid}` };
}

/**
 * Construct a gateway URL for retrieving a pinned document.
 * Used by the backend to provide a download link alongside the CID.
 */
export function gatewayUrl(cid: string): string {
  const gateway =
    process.env.PINATA_GATEWAY ?? "https://gateway.pinata.cloud";
  return `${gateway}/ipfs/${cid}`;
}
