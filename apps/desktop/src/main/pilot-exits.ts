// The three ways out of a pilot: submit for verification, attest complete, close as superseded.
//
// Beside `command.ts` rather than inside it, `limits.ts`'s reason: that file sits at the length the
// gate refuses, and these are one feature with one guard. Each is one POST to a route under the Job,
// and what comes back is the Job's row, folded onto the Board as `JobCommands.act` folds one.

import type { Outcome } from "@armada/protocol";
import type { PilotExit } from "../shared/api/sessions";
import type { Board } from "./command";
import { ask, CHECKS_MS, isJobSummary, route } from "./request";

const ROUTE: Record<PilotExit, string> = {
  submit: "submit_for_verification",
  attest: "attest_complete",
  supersede: "close_as_superseded",
};

/** What an exit needs of the connection: where Fleet answers, and the Board it folds onto. */
export type Reaches = Pick<Board, "port" | "fold" | "reread" | "refresh">;

export class PilotExits {
  private readonly board: Reaches;
  /** One exit of one Job at a time: a second press would run the Checks twice or answer the Job's own 409. */
  private readonly exiting = new Set<string>();

  constructor(board: Reaches) {
    this.board = board;
  }

  /**
   * Take one of the exits. **Submitting waits for the Checks**, as `rerunChecks` does, so it has their
   * bound; attesting and superseding carry the person's note where they gave one.
   */
  async exit(jobId: string, exit: PilotExit, note?: string): Promise<Outcome> {
    if (this.exiting.has(jobId)) return { ok: false, why: "already_piloting" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };
    this.exiting.add(jobId);
    try {
      const answer =
        exit === "submit"
          ? await ask(port, "POST", route(jobId, ROUTE.submit), undefined, CHECKS_MS)
          : await ask(port, "POST", route(jobId, ROUTE[exit]), note === undefined ? {} : { note });
      if (answer.ok !== true) return answer.outcome;
      // A body that is not a row would put a malformed one on the Board, so it is a re-read instead.
      if (isJobSummary(answer.body)) this.board.fold(answer.body);
      else await this.board.reread(port);
      this.board.refresh(port, jobId);
      return { ok: true };
    } finally {
      this.exiting.delete(jobId);
    }
  }
}
