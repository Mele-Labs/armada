// A Session's rows as its thread draws them, and a fork taken from words selected in one.

import { seedDraft } from "@armada/components";
import type { ForkPick, SessionThreadRow } from "@armada/components";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

export function threadRowsOf(session: Session): SessionThreadRow[] {
  return threadRowsFrom(session.rows);
}

/** The rows up to and including the one with this id; all of them with no id. **An id no row has gives none**: a cut that cannot be found must not show what came after it. */
export function upTo<T extends { id: string }>(rows: readonly T[], at: string | undefined): readonly T[] {
  if (at === undefined) return rows;
  const end = rows.findIndex((one) => one.id === at);
  return end < 0 ? [] : rows.slice(0, end + 1);
}

export function threadRowsFrom(rows: Session["rows"]): SessionThreadRow[] {
  return rows.map((row): SessionThreadRow => {
    if (row.kind === "lease" || row.kind === "tool" || row.kind === "handoff" || row.kind === "command" || row.kind === "compaction" || row.kind === "window" || row.kind === "started" ||row.kind === "check" || row.kind === "retro") return row;
    if (row.from.kind === "session") {
      return { id: row.id, at: row.at, kind: "message", from: "session", sender: { id: row.from.id, title: row.from.title }, text: row.text };
    }
    return {
      id: row.id,
      at: row.at,
      kind: "message",
      from: row.from.kind,
      text: row.text,
      ...(row.files === undefined ? {} : { files: row.files }),
      ...(row.sketches === undefined ? {} : { sketches: row.sketches }),
      ...(row.tags === undefined ? {} : { tags: row.tags }),
    };
  });
}

/** Fork at the message the words were selected in. The words are in the new Session's box before it opens, with the caret behind them. */
export async function forkAtPick(forkAt: NonNullable<SessionsDraft["forkAt"]>, id: string, { rowId, quote }: ForkPick, focus: (id: string) => void): Promise<string | undefined> {
  const forked = await forkAt(id, rowId);
  if (forked !== undefined) {
    seedDraft(forked, quote);
    focus(forked);
  }
  return forked;
}
