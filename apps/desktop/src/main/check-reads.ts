// A Session Check's output, and stopping a Session's running Check.
//
// Neither is held: a recorded output does not move, and a stop answers the caller. `port` arrives as
// a function so this file never learns how a port is found, only that one might not be.

import type { CheckOutputRead, Outcome } from "@armada/protocol";
import { sessionCheckOutputOf, stopSessionCheck } from "./request";

export class CheckReads {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  /**
   * One Check a Session's agent ran, by its run id, for the person who opened that Check.
   *
   * **It answers the caller and publishes nothing.** A recorded output does not move, and it is one
   * reader's gesture on one Check rather than state the window renders from. Nothing connected is
   * the caller's to say, so it comes back as the refusal every other operation here uses rather
   * than as silence.
   */
  async readSessionCheckOutput(run: number): Promise<CheckOutputRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    return await sessionCheckOutputOf(port, run);
  }

  /** Stop a Session's running Check, gracefully or by force. Answers the caller; the run's end arrives on the event stream. */
  async stopSessionCheck(run: number, force: boolean): Promise<Outcome> {
    const port = this.port();
    if (port === null) return { ok: false, why: "not_connected" };
    return await stopSessionCheck(port, run, force);
  }
}
