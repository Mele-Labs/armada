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
  /** `Kind: what`, a one or two word kind and a short what, or the kind alone: `Answered: Keep them in the store`, `Sent: to Fix the pane`, `Passed on`. */
  outcome: string;
  /** RFC 3339. */
  at: string;
  /** Who settled it: the Operator's ruling, or the person's press on a suggestion. */
  by: OperatorDecider;
  /** Why, in the words kept with the ruling. Absent where none were. */
  because?: string;
};

export type OperatorDecider = "operator" | "person";

/** One move of the Operator's master switch, kept so the Log can show where older rulings end. */
export type OperatorBoardSwitch = {
  /** What the switch was moved to. */
  on: boolean;
  /** RFC 3339. */
  at: string;
};

/** One thing a suggestion offers to do: `Do it` (choice 0), then `Skip` (choice 1). */
export type OperatorBoardOption = {
  label: string;
};

/** What the Operator would do with something no Job holds, such as a note, waiting on one press. */
export type OperatorBoardSuggestion = {
  /** `suggestion:<n>`, the id the Needs you rows use. `AnswerOperatorSuggestion.id` names it. */
  id: string;
  /** What the row says, as it was kept. */
  text: string;
  options: OperatorBoardOption[];
  /** When it was raised, RFC 3339. */
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
  /** The unsettled suggestions tied to no Job, oldest first. */
  suggestions: OperatorBoardSuggestion[];
  done: OperatorBoardSettled[];
  /** Present only while the week's plan is used to the share where the Operator pauses; both are percent. */
  paused?: {
    week: number;
    at: number;
  };
  /** The last twenty moves of the master switch, newest first. Absent while there are none. */
  switched?: OperatorBoardSwitch[];
};

/**
 * `move_on_operator_board` (`POST /operator/board/move`): `id` goes before `before` in `next`, last where `before` is
 * absent. Answers the `OperatorBoard`. 404 `fleet.operator_board_item_unknown` where either is not waiting.
 */
export type MoveOnOperatorBoard = {
  id: string;
  before?: string;
};

/**
 * `answer_operator_suggestion` (`POST /operator/suggestions/answer`): one press on a suggestion no Job holds, `choice` 0
 * for Do it or 1 for Skip. Answers the `OperatorBoard`. 404 `fleet.operator_suggestion_unknown` where `id` is none, or
 * a Job's own; 409 where it is settled already.
 */
export type AnswerOperatorSuggestion = {
  id: string;
  choice: number;
};
