// A background act that failed after its request was answered — `crates/ipc/src/act_failed.rs`.
//
// The refusal is the `WireError` a request's refusal answers with, so Bridge words it as it words one.
// The header rules in `protocol.ts` hold here: hand-written, and the set Fleet closes is written out.

import type { WireError } from "./index";

/** The acts that finish after their reply. */
export type BackgroundAct = "checkout_to_pull_request" | "commit_to_repository" | "pull_request_press";

/** `act.failed`: what failed, for which repository or Job, and the refusal. */
export type ActFailed = {
  act: BackgroundAct;
  /** The repository the act was for, where it had one. */
  manifest_id?: string;
  /** The Job the act was for, where it had one. */
  session_id?: string;
  error: WireError;
};
