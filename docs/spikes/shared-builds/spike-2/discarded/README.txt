Discarded runs, kept as the record of why.

1. First attempt at M-p4107-r1: the clone's directory mtimes were new, so ipc's build script
   (rerun-if-changed on directories) reran and the workspace rebuilt; stopped at the 900 s budget.

2. Second attempt at M-p4107-r1, finished: build 14m27s, 9 workspace crates compiled for a change
   that touches only crates/bridge-native, test step "narrowed to nothing it runs".
   Cause, from CARGO_LOG=cargo::core::compiler::fingerprint=info (../diag-ipc-fingerprint.log):
   crates/ipc/build.rs emits one `rerun-if-changed` per operations/*.toml as an absolute path
   (714 paths), so a target built at one worktree path is dirty at any other and ipc and everything
   above it rebuilds. The run is not discarded for being slow; it is discarded because it measured
   "target moved to a new path", not "this branch's change".
   Every later run uses ONE fixed worktree and target path per set and restores the target from a
   pristine clone before each run.
