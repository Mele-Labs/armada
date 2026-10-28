// A pull request's notes, as Fleet answers `/pull_request_notes/...`. Nothing is kept here: Fleet keeps them.

import type { Outcome, PullRequestNote, PullRequestNotes, SavePullRequestNote, SendPullRequestNotes } from "@armada/protocol";
import type { PullRequestNoteAnswer, PullRequestNotesAnswer } from "../shared/api/pull-request-notes";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class PullRequestNotesHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  private path(manifestId: string, number: number): string {
    return `/pull_request_notes/${encodeURIComponent(manifestId)}/${number}`;
  }

  async list(manifestId: string, number: number): Promise<PullRequestNotesAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", this.path(manifestId, number));
    return answer.ok === true ? { ok: true, value: answer.body as PullRequestNotes } : { ok: false, outcome: answer.outcome };
  }

  async save(manifestId: string, number: number, save: SavePullRequestNote): Promise<PullRequestNoteAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", this.path(manifestId, number), save);
    return answer.ok === true ? { ok: true, value: answer.body as PullRequestNote } : { ok: false, outcome: answer.outcome };
  }

  async remove(manifestId: string, number: number, id: string): Promise<PullRequestNotesAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", `${this.path(manifestId, number)}/${encodeURIComponent(id)}/remove`);
    return answer.ok === true ? { ok: true, value: answer.body as PullRequestNotes } : { ok: false, outcome: answer.outcome };
  }

  async send(manifestId: string, number: number, send: SendPullRequestNotes): Promise<PullRequestNotesAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", `${this.path(manifestId, number)}/sent`, send);
    return answer.ok === true ? { ok: true, value: answer.body as PullRequestNotes } : { ok: false, outcome: answer.outcome };
  }
}
