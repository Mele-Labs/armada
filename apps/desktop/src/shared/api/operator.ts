// The Operator page's reads and acts, and the board main keeps current. A slice imports protocol, never another slice;
// `../api.ts` and `../bridge.ts` compose them.
//
// Main answers each channel from Fleet's `/operator/*` routes (`main/operator.ts`); the mock answers them in a walk.

import type { Outcome, OperatorBoard, OperatorSaid, OperatorTalk } from "@armada/protocol";

/** The last twenty messages, or why they could not be read. */
export type OperatorTalkRead =
  | {
      ok: true;
      talk: OperatorTalk;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

/** What the Operator said back to a message, or the refusal. */
export type OperatorTalked =
  | {
      ok: true;
      said: OperatorSaid;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

/** The board as a move left it, or the refusal. */
export type OperatorBoardMoved =
  | {
      ok: true;
      board: OperatorBoard;
    }
  | {
      ok: false;
      outcome: Outcome;
    };

export type OperatorApi = {
  /** The last twenty messages in the box, oldest first. */
  readOperatorTalk: () => Promise<OperatorTalkRead>;
  /** One message to the Operator; `about` is the board item or Needs you item the person is talking about. */
  talkToOperator: (words: string, about?: string) => Promise<OperatorTalked>;
  /** `id` goes before `before` in Up next, last where `before` is absent. Publishes the board it answers. */
  moveOnOperatorBoard: (id: string, before?: string) => Promise<OperatorBoardMoved>;
  /** Do it (0) or Skip (1) on a suggestion no Job holds. Do it waits for the act. Publishes the board it answers. */
  answerOperatorSuggestion: (id: string, choice: 0 | 1) => Promise<OperatorBoardMoved>;
};

export type OperatorState = {
  /** `null` until read. Main reads it on connecting and again on every `operator.board_changed`. */
  operatorBoard: OperatorBoard | null;
  /** Moves on every connection and every `operator.talk_changed`: the Operator page reads the talk again when it does. */
  operatorTalkMoved: number;
};

export const OPERATOR_NOTHING_YET: OperatorState = {
  operatorBoard: null,
  operatorTalkMoved: 0,
};

export const OPERATOR_CHANNELS = {
  readOperatorTalk: "bridge:read-operator-talk",
  talkToOperator: "bridge:talk-to-operator",
  moveOnOperatorBoard: "bridge:move-on-operator-board",
  answerOperatorSuggestion: "bridge:answer-operator-suggestion",
} as const;
