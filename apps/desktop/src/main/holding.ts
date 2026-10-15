// What Fleet is holding disk for, read while somebody is deciding about it.
//
// Beside `connection.ts` rather than inside it, for `reports.ts`'s reason: the
// connection is a socket, a runtime file and a state machine, and a GET is none
// of those. No port is held, only passed in, so nothing here can drift from
// what the socket believes.
//
// # It is not a `JobReader`, and that is the whole shape of it
//
// Every read in `reader.ts` is scoped to a job, and the rule that file exists
// for is that an answer whose job moved while it was in flight is dropped
// rather than painted onto the job that replaced it. There is no id here for
// that rule to check: what is being decided is which of a set to give back, and
// that is a question about the set. What survives of the rule is the half that
// still applies — an answer landing after the surface closed is dropped.
//
// # Re-read after a reclaim, never folded
//
// Reclaiming changes what is held, and the answer to `reclaim_worktree` says
// only what happened to the one job. Folding that receipt into this list would
// be Bridge deciding a worktree is gone on its own reading; asking again is
// Fleet answering. The list is short, the read is cheap, and the correction is
// the point — a row that fails to go is a row that has to stay.
//
// # And again when Fleet enters a lease or a release
//
// A Session's first write takes a slot with no act of Bridge's, and the pool was read only when a surface
// opened or an act re-read it, so the bays stayed as drawn. `timeline.appended` carries `slot.leased` and
// `slot.released`, and `arrivals.ts` reads the list again on either: no new poll, and nothing where nothing is open.
//
// # The last list stays, and is drawn before the next read answers
//
// A surface that reopens draws the list read last at once and the fresh one replaces it, because
// the read takes seconds on a busy machine and a blank page for that long is the cost this
// avoids. "Reading" is only for a list never read. A failed read leaves the list up rather than
// replacing it with a refusal, and one read is made when Fleet is first reached (`warm`) so the
// first open after a restart has a list too.
//
// # This is `reports.ts` again, and knowingly
//
// Two reads on this seam are scoped to nothing and held while a surface is
// open, and they move through the same four states. They are two classes rather
// than one generic because a third does not exist yet and the shape a generic
// would need is uglier than the duplication; the day there is a third, they
// collapse.

import type { HeldWorktrees, WorktreesHeld } from "@armada/protocol";
import { ask } from "./request";

/** One read of `GET /worktrees`, held for as long as a surface wants it. */
export class HeldReader {
  private readonly publish: (held: HeldWorktrees) => void;
  /** Whether anybody is deciding. The only scope this read has. */
  private open = false;
  /** The last list Fleet gave, kept past the surface that asked for it. Null until there has been one. */
  private last: WorktreesHeld | null = null;
  /** Which read is the latest, so an answer that a newer read overtook is dropped. */
  private generation = 0;

  constructor(publish: (held: HeldWorktrees) => void) {
    this.publish = publish;
  }

  /** Read them, or stop reading. The list read last stays, and is the first thing a reopened surface draws. */
  async want(port: number | null, want: boolean): Promise<void> {
    this.open = want;
    if (!want) return;
    if (this.last !== null) this.publish({ state: "read", held: this.last });
    if (port === null) {
      if (this.last === null) this.publish({ state: "failed", outcome: { ok: false, why: "not_connected" } });
      return;
    }
    await this.again(port);
  }

  /**
   * Read again, where a surface has one open. Nothing open is no read.
   *
   * **A list already on screen stays there while the next read is out**, and stays if the read
   * fails. The pool is read again on a timer while a Scout reads, and "reading" would blank the
   * surface and its refusals every time.
   */
  async again(port: number): Promise<void> {
    if (!this.open) return;
    await this.read(port);
  }

  /** One read for nobody, once, so the first surface to open finds a list. Nothing is read where one is held. */
  async warm(port: number): Promise<void> {
    if (this.last !== null) return;
    await this.read(port);
  }

  private async read(port: number): Promise<void> {
    const mine = ++this.generation;
    if (this.open && this.last === null) this.publish({ state: "reading" });
    const answer = await ask(port, "GET", "/worktrees");
    if (mine !== this.generation) return;
    if (answer.ok === true) this.last = answer.body as WorktreesHeld;
    // The surface closed while this was in flight, so nobody is reading the
    // answer and publishing it would draw a list onto a screen that is gone.
    // It is kept: the next surface to open starts from it.
    if (!this.open) return;
    if (this.last !== null) this.publish({ state: "read", held: this.last });
    else if (answer.ok !== true) this.publish({ state: "failed", outcome: answer.outcome });
  }

  /** The read ends with the window. Nothing is published: the surface is gone. */
  close(): void {
    this.open = false;
  }
}
