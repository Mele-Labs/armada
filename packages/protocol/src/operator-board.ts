// The Operator's board — Bridge's mirror of `crates/ipc/src/operator_board.rs`.
// `crates/ipc/operations/get_operator_board.toml`.

/**
 * What kind of thing an item is. A plan and a Job's start are both `start`; an issue, a review request, a red main
 * and a Later item are each a `report`.
 */
export type OperatorBoardKind = "start" | "question" | "card" | "stall" | "report";

/** One arrival the Operator has not ruled on. */
export type OperatorBoardItem = {
  /** The arrival's id: what `MoveOnOperatorBoard` and `TalkToOperator.about` name it by. */
  id: string;
  kind: OperatorBoardKind;
  session_id?: string;
  /** What a person reads: the Job's title, or the question's first line. */
  title: string;
  /** The repository's name. */
  repository?: string;
  /** How long it has been with the Operator, such as `12s` or `3m`. */
  waits?: string;
};

/** One ruling the Operator made. */
export type OperatorBoardSettled = {
  id: string;
  kind: OperatorBoardKind;
  session_id?: string;
  title: string;
  repository?: string;
  /** `Kind: what`, a one or two word kind and a short what, or the kind alone: `Answered: Keep them in the store`, `Passed on`. */
  outcome: string;
  /** RFC 3339. */
  at: string;
};

/**
 * `get_operator_board` (`GET /operator/board`): whether the Operator is on, the arrival a call is out on, those
 * waiting for it in the order they will be taken, and its last twenty rulings, newest first. What a call has been handed
 * leads `next` and cannot be moved.
 */
export type OperatorBoard = {
  on: boolean;
  now?: OperatorBoardItem;
  next: OperatorBoardItem[];
  done: OperatorBoardSettled[];
};

/**
 * `move_on_operator_board` (`POST /operator/board/move`): `id` goes before `before` in `next`, last where `before` is
 * absent. Answers the `OperatorBoard`. 404 `fleet.operator_board_item_unknown` where either is not waiting.
 */
export type MoveOnOperatorBoard = {
  id: string;
  before?: string;
};
