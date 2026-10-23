// The Logbook as Fleet serves it: every repository's entries, the owner's habits and this Mac's, read into one list, and
// the owner's acts on an entry. Nothing is kept here; the page asks again when it opens.
//
// `GET /logbook` bare answers the habits and this Mac's alone, so each served repository is asked by its root and the
// answers are joined in the order Fleet lists the repositories. `docs/concepts/logbook.md`.

import type { Logbook, LogbookAnswer, LogbookEntry, LogbookForgotten, LogbookRead, LogbookTidied, Outcome, LogbookTidy, TidiedLogbook, RepositoryList } from "@armada/protocol";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

const entryPath = (id: string, act: "edit" | "lock" | "forget") => `/logbook/entries/${encodeURIComponent(id)}/${act}`;

export class LogbookHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async read(): Promise<LogbookRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const listed = await ask(port, "GET", "/repositories");
    if (listed.ok !== true) return { ok: false, outcome: listed.outcome };
    const roots = (listed.body as RepositoryList).repositories.map((one) => one.root);
    const paths = [
      "/logbook",
      ...roots.map((root) => `/logbook?section=repo&repository=${encodeURIComponent(root)}`),
    ];
    const answers = await Promise.all(paths.map((path) => ask(port, "GET", path)));
    const entries: LogbookEntry[] = [];
    let tidy: LogbookTidy = { running: false };
    // Habits and this Mac's come first in the answer; the page orders its panels itself.
    for (const answer of answers) {
      if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
      entries.push(...(answer.body as Logbook).entries);
      // Tidy is Fleet-wide, so any answer says the same; the first is the habits' and this Mac's.
      if (answer === answers[0]) tidy = (answer.body as Logbook).tidy;
    }
    return { ok: true, entries, tidy };
  }

  async edit(id: string, text: string): Promise<LogbookAnswer> {
    return await this.act(entryPath(id, "edit"), { text });
  }

  async lock(id: string, locked: boolean): Promise<LogbookAnswer> {
    return await this.act(entryPath(id, "lock"), { locked });
  }

  async forget(id: string): Promise<LogbookForgotten> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", entryPath(id, "forget"), {});
    return answer.ok === true ? { ok: true } : { ok: false, outcome: answer.outcome };
  }

  /** Runs Tidy over every repository's Logbook, the habits and this Mac's. */
  async tidy(): Promise<LogbookTidied> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/logbook/tidy", {});
    return answer.ok === true ? { ok: true, running: (answer.body as TidiedLogbook).running } : { ok: false, outcome: answer.outcome };
  }

  private async act(path: string, body: unknown): Promise<LogbookAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", path, body);
    return answer.ok === true ? { ok: true, entry: answer.body as LogbookEntry } : { ok: false, outcome: answer.outcome };
  }
}
