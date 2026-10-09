/**
 * src/indexer/index.ts — Soroban event indexer (implemented in Day 6)
 *
 * Polls Soroban RPC `getEvents` for the event topics emitted by the contract:
 *   TitleRegistered, TransferInitiated, TransferCoSigned,
 *   TransferExecuted, TransferCancelled, DisputeFlagged, DisputeResolved
 *
 * Writes events into a Postgres read-cache (`titles`, `transfer_proposals`,
 * `events` tables). This DB is explicitly NOT the source of truth — the chain
 * is. The indexer only materialises chain state for fast querying. All
 * security-critical checks must go directly to the chain.
 *
 * See README "Development workflow" and .env.example for configuration.
 */
export {};
