# Protocol 13.38: a workspace's Command runs where the root has no Manifest

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`?repository=<root>` on `start_checkout_run`, `undo_checkout_run`, `list_checkout_runs`, `get_checkout_run_output` and `get_checkout_run_diff`, refused beside `?manifest_id=` as Verify's routes are (#986).
`StartCheckoutRun.workspace`, optional, names a directory whose own `armada.yml` declares the Command.
`CheckoutRunSheet.workspaces` lists each workspace's Commands, and `workspace` rides on `CheckoutRunUnderway` and `CheckoutRunRecord`.
All additive.
