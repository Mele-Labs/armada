# Protocol 13.29: findings a person dismissed

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`ipc::DismissedRow`, additive on `JobConfidence` as `dismissed`, and `dismiss_finding`'s body `ipc::FindingDismissed` (#907).
A dismissed finding leaves `needs_you`, `small_fixes` and `for_context` and is listed in `dismissed` with the reason a person gave.
**Left out where empty**, which is every Job nobody dismissed anything on.
