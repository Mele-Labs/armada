// The replies a Job's quick bar is offered beyond his saved lines. `crates/ipc/src/session_quick.rs`,
// `docs/practices/protocol/a-jobs-quick-replies.md`.
//
// The header rules in `protocol.ts` hold here. Nothing is published: Bridge reads again when the Job's last row changes.

/**
 * `GET /sessions/quick?session_id=`. Each list is empty where there is nothing to offer, and both are where `thread.look`
 * hides the bar for the Job; `learnt` is empty where learning is off. 422 `fleet.no_such_session` for a Job Fleet does not host.
 */
export type SessionQuick = {
  /** At most three short answers to the agent's last message. Empty while the agent has not spoken last. */
  asked: string[];
  /** At most six lines he sends often across every Job, the most useful first. */
  learnt: string[];
};
