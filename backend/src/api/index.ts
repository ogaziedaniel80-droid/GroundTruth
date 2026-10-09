/**
 * src/api/index.ts — REST API (implemented in Day 6)
 *
 * Endpoints needed by the frontend (per README "System architecture"):
 *   GET  /titles/:id             — title metadata (from indexer read-cache)
 *   GET  /titles/:id/history     — event history for a title
 *   GET  /transfers/pending      — pending transfer proposals (registrar dashboard)
 *   POST /titles/hash-check      — convenience wrapper around on-chain verify_hash
 *
 * NOTE: The frontend must never use the REST API for security-critical
 * verification. It always recomputes the hash client-side and calls
 * `verify_hash` directly on-chain. See README "Development workflow".
 */
export {};
