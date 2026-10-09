/**
 * src/storage/index.ts
 *
 * Re-exports for the storage layer.
 */
export { hashDocument, sha256Hex, sha256Bytes } from "./hash.js";
export { pinToIpfs, gatewayUrl } from "./ipfs.js";
export type { PinResult } from "./ipfs.js";
