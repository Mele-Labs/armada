// The Operator as Fleet serves it: its board, the box's messages and a message to it. The board is pushed state, so a
// move publishes the board it answers; a message and the read of the box are asked each time.

import type {
  AnswerOperatorSuggestion,
  MoveOnOperatorBoard,
  OperatorBoard,
  OperatorSaid,
  OperatorTalk,
  Outcome,
  TalkToOperator,
} from "@armada/protocol";

import type { OperatorBoardMoved, OperatorTalked, OperatorTalkRead } from "../shared/api/operator";
import type { BridgeState } from "../shared/bridge";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = {
  ok: false,
  why: "not_connected",
};

/**
 * The board Fleet holds. **A failed read publishes nothing**, so the board last read stands: `null` is only "not read yet".
 * `operator.board_changed` carries no body, so this is what answers it.
 */
export async function readOperatorBoard(
  port: number,
  publish: (change: Partial<BridgeState>) => void,
): Promise<void> {
  const answer = await ask(port, "GET", "/operator/board");
  if (answer.ok === true)
    publish({
      operatorBoard: answer.body as OperatorBoard,
    });
}

/** How often the conversation has moved since Bridge started, which the Operator page reads the talk again on. */
let talkMoved = 0;

/** The Operator's conversation is not what it was: the page reads it again. `operator.talk_changed` carries nothing. */
export function operatorTalkMoved(publish: (change: Partial<BridgeState>) => void): void {
  talkMoved += 1;
  publish({
    operatorTalkMoved: talkMoved,
  });
}

export class OperatorHost {
  private readonly port: () => number | null;
  private readonly publish: (change: Partial<BridgeState>) => void;
  /** The window came to the front while Fleet was not connected: told once it is. */
  private inFrontOwed = false;

  constructor(port: () => number | null, publish: (change: Partial<BridgeState>) => void) {
    this.port = port;
    this.publish = publish;
  }

  /** Bridge's window came to the front. Fleet decides whether it is the day's first, and briefs then; nothing is answered here. */
  inFront(): void {
    const port = this.port();
    this.inFrontOwed = port === null;
    if (port !== null) void ask(port, "POST", "/operator/front", {});
  }

  /** Fleet connected: a time in front it missed is told now. */
  connected(): void {
    if (this.inFrontOwed) this.inFront();
  }

  async readTalk(): Promise<OperatorTalkRead> {
    const port = this.port();
    if (port === null)
      return {
        ok: false,
        outcome: NOT_CONNECTED,
      };
    const answer = await ask(port, "GET", "/operator/talk");
    return answer.ok === true
      ? {
          ok: true,
          talk: answer.body as OperatorTalk,
        }
      : {
          ok: false,
          outcome: answer.outcome,
        };
  }

  async talk(words: string, about?: string): Promise<OperatorTalked> {
    const port = this.port();
    if (port === null)
      return {
        ok: false,
        outcome: NOT_CONNECTED,
      };
    const body: TalkToOperator = {
      words,
      ...(about === undefined
        ? {}
        : {
            about,
          }),
    };
    const answer = await ask(port, "POST", "/operator/talk", body);
    return answer.ok === true
      ? {
          ok: true,
          said: answer.body as OperatorSaid,
        }
      : {
          ok: false,
          outcome: answer.outcome,
        };
  }

  async move(id: string, before?: string): Promise<OperatorBoardMoved> {
    const port = this.port();
    if (port === null)
      return {
        ok: false,
        outcome: NOT_CONNECTED,
      };
    const body: MoveOnOperatorBoard = {
      id,
      ...(before === undefined
        ? {}
        : {
            before,
          }),
    };
    const answer = await ask(port, "POST", "/operator/board/move", body);
    if (answer.ok !== true)
      return {
        ok: false,
        outcome: answer.outcome,
      };
    const board = answer.body as OperatorBoard;
    this.publish({
      operatorBoard: board,
    });
    return {
      ok: true,
      board,
    };
  }

  /** One press on a suggestion no Job holds. Do it waits for the act, a Job's start included. Publishes the board it answers. */
  async answerSuggestion(id: string, choice: number): Promise<OperatorBoardMoved> {
    const port = this.port();
    if (port === null)
      return {
        ok: false,
        outcome: NOT_CONNECTED,
      };
    const body: AnswerOperatorSuggestion = {
      id,
      choice,
    };
    const answer = await ask(port, "POST", "/operator/suggestions/answer", body);
    if (answer.ok !== true)
      return {
        ok: false,
        outcome: answer.outcome,
      };
    const board = answer.body as OperatorBoard;
    this.publish({
      operatorBoard: board,
    });
    return {
      ok: true,
      board,
    };
  }
}
