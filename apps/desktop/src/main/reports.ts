// Every report a person has filed, and the counts they are read beside.
//
// Beside `connection.ts` rather than inside it, for the reason `reader.ts`,
// `review.ts` and `observe.ts` are: the connection is a socket, a runtime file
// and a state machine, and a GET is none of those. No port is held, only passed
// in, so nothing here can drift from what the socket believes.
//
// # It is not a `JobReader`, and the difference is the point
//
// Every other read on this side is scoped to a Job, and the rule that makes
// `reader.ts` worth having is that an answer whose Job moved while it was in
// flight is dropped rather than painted onto the Job that replaced it. There is
// no id here for that rule to check: `list_reports` is deliberately not scoped
// to a Job, because a report outlives the Job it is about — `armada clean`
// forgets the Job and the report stays whole — so a listing reachable only
// through a Job would lose exactly the reports that most need reading.
//
// What is left of the rule is the half that still applies: an answer that lands
// after the surface closed is dropped, because publishing it would put a list
// back on a screen nobody is looking at.
//
// # The bodies travel with the list, so it is read when asked for, and the last one stays
//
// `crates/ipc/operations/` states the cost: one rendered record per report,
// all of them in one answer, because the record is the payload the sentence is
// a finding about. That is affordable while the list is read deliberately, which
// is why it is asked for and not polled or warmed. The one list read last is kept
// past the surface that asked, and a reopened surface draws it while the next read
// is out: "reading" is only for a list never read, and a failed read leaves the
// list up rather than replacing it with a refusal.

import type { Reports } from "@armada/protocol";
import type { ReportList } from "@armada/protocol";
import { ask } from "./request";

/** One read of `GET /reports`, held for as long as a surface wants it. */
export class ReportsReader {
  private readonly publish: (reports: Reports) => void;
  /** Whether anybody is reading. The only scope this read has. */
  private open = false;
  /** The last list Fleet gave, kept past the surface that asked for it. Null until there has been one. */
  private last: ReportList | null = null;
  /** Which read is the latest, so an answer that a newer read overtook is dropped. */
  private generation = 0;

  constructor(publish: (reports: Reports) => void) {
    this.publish = publish;
  }

  /** Read them, or stop reading. The list read last stays, and is the first thing a reopened surface draws. */
  async want(port: number | null, want: boolean): Promise<void> {
    this.open = want;
    if (!want) return;
    if (this.last !== null) this.publish({ state: "read", list: this.last });
    if (port === null) {
      if (this.last === null) this.publish({ state: "failed", outcome: { ok: false, why: "not_connected" } });
      return;
    }
    await this.again(port);
  }

  /** Read again, where a surface has one open. Nothing open is no read. */
  async again(port: number): Promise<void> {
    if (!this.open) return;
    const mine = ++this.generation;
    if (this.last === null) this.publish({ state: "reading" });
    const answer = await ask(port, "GET", "/reports");
    if (mine !== this.generation) return;
    if (answer.ok === true) this.last = answer.body as ReportList;
    // The surface closed while this was in flight, so nobody is reading the
    // answer and publishing it would draw a list onto a screen that is gone.
    // It is kept: the next surface to open starts from it.
    if (!this.open) return;
    if (this.last !== null) this.publish({ state: "read", list: this.last });
    else if (answer.ok !== true) this.publish({ state: "failed", outcome: answer.outcome });
  }

  /** The read ends with the window. Nothing is published: the surface is gone. */
  close(): void {
    this.open = false;
  }
}
