/**
 * src/storage/hash.ts
 *
 * SHA-256 document hashing for GroundTruth.
 *
 * The contract stores `doc_hash: BytesN<32>` (a 32-byte value on-chain).
 * This module hashes a raw document buffer with SHA-256, producing exactly
 * 32 bytes. The hex string form is used as the `doc_hash` argument when
 * building `register_title` transactions.
 *
 * Documents themselves NEVER go on-chain — only their hashes do. See README
 * "Trust & security model": "Documents and PII never touch the chain."
 */

import { createHash } from "node:crypto";

/** SHA-256 of `data`, returned as a lowercase hex string (64 chars = 32 bytes). */
export function sha256Hex(data: Buffer | Uint8Array): string {
  return createHash("sha256").update(data).digest("hex");
}

/**
 * SHA-256 of `data` as a `Uint8Array` (32 bytes).
 * This is the format the Soroban `BytesN<32>` XDR type expects when building
 * a transaction with the stellar-sdk.
 */
export function sha256Bytes(data: Buffer | Uint8Array): Uint8Array {
  return createHash("sha256").update(data).digest();
}

/**
 * Hash a document buffer and return both the hex string (for display/storage)
 * and the raw bytes (for the Soroban transaction builder).
 */
export function hashDocument(data: Buffer | Uint8Array): {
  hex: string;
  bytes: Uint8Array;
} {
  const bytes = sha256Bytes(data);
  return { hex: Buffer.from(bytes).toString("hex"), bytes };
}
