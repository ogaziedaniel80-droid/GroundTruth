# GroundTruth Contract — Error Code Reference

Every `ContractError` variant, its numeric discriminant, and the conditions
under which it is raised. This is the authoritative reference for integrators
building on the contract and for contributors writing tests.

---

| Code | Name | When it fires |
|------|------|---------------|
| 1 | `NotAuthorized` | The caller is not permitted to perform the requested action. For `register_title`, `flag_dispute`, `resolve_dispute`, and `co_sign_transfer` this means the caller is not in the registrar set. For `initiate_transfer` it means the caller is not the current title owner. For `cancel_transfer` it means the caller is neither the proposer nor the admin. |
| 2 | `AlreadyInitialized` | `initialize` was called on a contract that already has an `Admin` key in instance storage. The contract can only be initialized once. |
| 3 | `NotInitialized` | A function that requires a live contract was called before `initialize`. Also returned by `add_registrar`, `remove_registrar`, `set_threshold`, and `cancel_transfer` when the `Admin` key is absent. |
| 4 | `TitleNotFound` | `get_title`, `verify_hash`, `initiate_transfer`, `execute_transfer`, `cancel_transfer`, `flag_dispute`, or `resolve_dispute` was called with a `title_id` that has no corresponding `TitleRecord` in persistent storage. |
| 5 | `TitleAlreadyExists` | `register_title` was called with an `id` that already exists in persistent storage. Title IDs are permanent and immutable once anchored. |
| 6 | `InvalidThreshold` | Raised in three places: `initialize` when `threshold == 0` or `threshold > registrars.len()`; `set_threshold` under the same conditions; and `initialize` when the registrar list is empty. |
| 7 | `RegistrarAlreadyExists` | `add_registrar` was called with an address already present in the registrar set. |
| 8 | `RegistrarNotFound` | `remove_registrar` was called with an address not in the registrar set. |
| 9 | `DuplicateApproval` | `co_sign_transfer` was called by a registrar who has already submitted an approval for this proposal. Each registrar counts once toward the threshold. |
| 10 | `ProposalExpired` | `co_sign_transfer` was called after the proposal's `expires_at` ledger timestamp. The proposal must be cancelled and a new one initiated. |
| 11 | `ProposalNotFound` | `co_sign_transfer`, `execute_transfer`, or `cancel_transfer` was called for a `title_id` that has no pending `TransferProposal` in persistent storage. |
| 12 | `InvalidStatusTransition` | Raised in two contexts: (a) `initiate_transfer` when the title is not `Active` (e.g. already `PendingTransfer`, `Disputed`, or `Revoked`), or a proposal is already present for that title; (b) `resolve_dispute` when the requested `new_status` is `Disputed` or `PendingTransfer` — a resolution must produce a definitive outcome (`Active` or `Revoked`). |

---

## Notes for integrators

- Errors are encoded as `u32` in XDR. The numeric codes above are stable and
  match the `#[repr(u32)]` attribute in `errors.rs`.
- The generated TypeScript bindings surface these as typed `ContractError`
  variants — prefer those over matching raw numbers in client code.
- `NotAuthorized` (1) and `InvalidStatusTransition` (12) are the two errors
  most likely to surface in UI flows; both indicate the caller needs to re-read
  the current on-chain state before retrying.
