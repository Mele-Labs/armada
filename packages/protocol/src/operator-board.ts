// The Operator's events — Bridge's mirror of `crates/ipc/src/operator_board.rs`.
// `crates/ipc/operations/operator.board_changed.toml`.

/** What became of a dispatch the Operator held: a Job started on it, folded into a live Job, waiting on the person's card, or kept for later as an idea. */
export type DispatchWay = "started" | "handed" | "asked" | "parked";

/**
 * `operator.dispatched`: what the Operator did with a dispatch it held (`dispatch_job`'s `operator`): started a Job,
 * handed it to a live one, kept it for later (`parked`, `dispatch_id` then the idea's id) or asked him. `session_id` and
 * `title` name the Job started or the live Job the work was folded into, and are absent while `asked`.
 */
export type OperatorDispatched = {
  dispatch_id: string;
  manifest_id: string;
  way: DispatchWay;
  session_id?: string;
  title?: string;
};
