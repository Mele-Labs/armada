// A Session's rows as its thread draws them, and a fork taken from words selected in one.

import { seedDraft } from "@armada/components";
import type { ForkPick, SessionThreadRow, SketchAnswer } from "@armada/components";
import type { AnswerSketch } from "@armada/protocol";
import type { AnswerDocument, Session, SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { fileCallOf, sketchCallOf } from "@armada/screens/src/sessions-wire";
import { sceneOfSketch } from "./session-sketch";

type SketchCard = Extract<SessionThreadRow, { kind: "sketch" }>;
type DocumentCard = Extract<SessionThreadRow, { kind: "document" }>;

export function threadRowsOf(session: Session): SessionThreadRow[] {
  const rows = threadRowsFrom(session.rows, session.attachments);
  // Fleet's line for a Job whose agent never started, after the message it was sent for.
  if (session.startFailed === undefined) return rows;
  return [...rows, { id: "not-started", at: rows[rows.length - 1]?.at ?? session.lastTurnAt ?? new Date(0).toISOString(), kind: "not_started", why: `Did not start: ${session.startFailed.cause}`, log: session.startFailed.log }];
}

/**
 * What a press on a sketch's card sends: Approve is `choice: 0`, Send back its note. **`drawn` is the drawing the ledger holds
 * now**, which Fleet checks against the sketch as it stands; one the ledger never gave is sent empty, which Fleet refuses.
 */
export function sketchAnswerOf(session: Pick<Session, "id" | "attachments">, sketch: string, answer: SketchAnswer): AnswerSketch {
  const held = session.attachments.find((one) => one.kind === "sketch" && one.id === sketch);
  const drawn = held?.kind === "sketch" ? (held.drawn ?? "") : "";
  const base = { session_id: session.id, sketch, drawn };
  return "text" in answer ? { ...base, how: "sent_back", words: answer.text } : { ...base, how: "approved" };
}

/** What a press on a document's Approve or Send back sends: the answer names the newest revision, the one put to him. */
export function documentAnswerOf(session: Pick<Session, "id" | "attachments">, page: string, answer: SketchAnswer): AnswerDocument {
  const held = session.attachments.find((one) => one.kind === "artifact" && one.id === page);
  const revision = held?.kind === "artifact" ? (held.revisions?.length ?? 0) : 0;
  const base = { session_id: session.id, page, revision };
  return "text" in answer ? { ...base, how: "sent_back", words: answer.text } : { ...base, how: "approved" };
}

/** The rows up to and including the one with this id; all of them with no id. **An id no row has gives none**: a cut that cannot be found must not show what came after it. */
export function upTo<T extends { id: string }>(rows: readonly T[], at: string | undefined): readonly T[] {
  if (at === undefined) return rows;
  const end = rows.findIndex((one) => one.id === at);
  return end < 0 ? [] : rows.slice(0, end + 1);
}

/**
 * The card a `show_sketch` call is, where the thread carries the call as its tool row. **Only a call the ledger
 * holds a sketch for draws one**: Fleet keeps the sketch it accepted under the call's title, so a call it refused
 * is no card, and neither is one whose scene would not draw.
 */
function sketchCardOf(row: { id: string; at: string; text: string }, ledger: readonly SessionAttachment[]): SketchCard | undefined {
  const call = sketchCallOf(row.text);
  if (call === undefined) return undefined;
  const sketch = `sketch:${call.title}`;
  if (!ledger.some((one) => one.kind === "sketch" && one.id === sketch)) return undefined;
  return { id: row.id, at: row.at, kind: "sketch", sketch, title: call.title, drawing: call.scene, ...(call.against === undefined ? {} : { against: call.against }) };
}

/**
 * The row a `show_file` call is, where the ledger holds the file. **Opened by the ledger's own path**, the canonical one Fleet
 * kept: the call's text is the path as the agent gave it, which may be relative, and main opens only a path the ledger names.
 * A call whose file the ledger does not hold stays the plain tool row it was.
 */
function fileRowOf(row: { id: string; at: string; text: string }, ledger: readonly SessionAttachment[]): Extract<SessionThreadRow, { kind: "file" }> | undefined {
  const call = fileCallOf(row.text);
  if (call === undefined) return undefined;
  const given = call.path.replace(/^\.\//, "");
  const held = ledger.find((one) => one.kind === "artifact" && one.form === "file" && (one.id === call.path || one.id.endsWith(`/${given}`)));
  return held?.kind === "artifact" ? { id: row.id, at: row.at, kind: "file", path: held.id, name: held.title } : undefined;
}

/** A write the agent made, as its tool row carries it: the tool's name, then the path. */
const WRITE_CALL = /^(?:Write|Edit|MultiEdit)\s+(\S+)\s*$/;

/**
 * The card a write of a page is, where the ledger holds the page as a document. Its revision is numbered after, by
 * `withDocumentState`. **A write of any other file stays the plain tool row it was.**
 */
function documentCardOf(row: { id: string; at: string; text: string }, ledger: readonly SessionAttachment[]): DocumentCard | undefined {
  const found = WRITE_CALL.exec(row.text);
  if (found === null) return undefined;
  const given = found[1]!.replace(/^\.\//, "");
  const held = ledger.find((one) => one.kind === "artifact" && one.form === "document" && (one.id === given || one.id.endsWith(`/${given}`)));
  return held?.kind === "artifact" ? { id: row.id, at: row.at, kind: "document", page: held.id, title: held.title, revision: 0 } : undefined;
}

/**
 * Each write of a page is its next revision. **The newest is the one put to him**: it asks until he answers it, and an
 * earlier one he answered keeps how it ended. An earlier one he never answered was replaced before he did.
 */
function withDocumentState(rows: SessionThreadRow[], ledger: readonly SessionAttachment[]): SessionThreadRow[] {
  const counted = new Map<string, number>();
  const numbered = rows.map((row): SessionThreadRow => {
    if (row.kind !== "document") return row;
    const revision = (counted.get(row.page) ?? 0) + 1;
    counted.set(row.page, revision);
    return { ...row, revision };
  });
  return numbered.map((row): SessionThreadRow => {
    if (row.kind !== "document") return row;
    const held = ledger.find((one) => one.kind === "artifact" && one.id === row.page);
    const answered = held?.kind === "artifact" ? held.answers?.find((one) => one.revision === row.revision) : undefined;
    const settled = answered === undefined ? undefined : { how: answered.how, at: answered.at, ...(answered.words === undefined ? {} : { words: answered.words }) };
    if (settled !== undefined) return { ...row, settled };
    if (counted.get(row.page) !== row.revision) return { ...row, replaced: true as const };
    return held?.kind === "artifact" && held.asked === true ? { ...row, asked: true as const } : row;
  });
}

/**
 * A card is replaced where a later card holds the same sketch. What it was drawn against is the latest card before it
 * with the title it names, as it was drawn then, else the sketch the ledger keeps the earlier scene of.
 */
function withSketchState(rows: SessionThreadRow[], ledger: readonly SessionAttachment[]): SessionThreadRow[] {
  const last = new Map(rows.flatMap((row) => (row.kind === "sketch" ? [[row.sketch, row.id] as const] : [])));
  return rows.map((row, at): SessionThreadRow => {
    if (row.kind !== "sketch") return row;
    const before = row.against === undefined ? undefined : rows.slice(0, at).reverse().find((one) => one.kind === "sketch" && one.title === row.against);
    const held = ledger.find((one) => one.kind === "sketch" && one.id === row.sketch);
    const kept = held?.kind === "sketch" && held.against !== undefined && held.against.title === row.against ? sceneOfSketch(held.against.drawing) : undefined;
    const earlier = before?.kind === "sketch" ? before.drawing : kept;
    const latest = last.get(row.sketch) === row.id;
    // What he was asked, and how he answered, belong to the sketch as it stands: the latest card holds them.
    const answer = !latest || held?.kind !== "sketch" ? {} : { ...(held.asked === true ? { asked: true as const } : {}), ...(held.settled === undefined ? {} : { settled: held.settled }) };
    return {
      ...row,
      ...(earlier === undefined ? {} : { againstScene: earlier }),
      ...(latest ? {} : { replaced: true as const }),
      ...answer,
    };
  });
}

export function threadRowsFrom(rows: Session["rows"], ledger: readonly SessionAttachment[] = []): SessionThreadRow[] {
  return withDocumentState(withSketchState(rowsDrawn(rows, ledger), ledger), ledger);
}

function rowsDrawn(rows: Session["rows"], ledger: readonly SessionAttachment[]): SessionThreadRow[] {
  // A browser reminder is the Job browser's own card in the thread (`job-browser/JobBrowserPane.tsx`), not a row.
  return rows.flatMap((row) => (row.kind === "browser_reminder" ? [] : [row])).map((row): SessionThreadRow => {
    if (row.kind === "tool") return sketchCardOf(row, ledger) ?? documentCardOf(row, ledger) ?? fileRowOf(row, ledger) ?? row;
    if (row.kind === "requested" || row.kind === "saved") return row;
    if (row.kind === "lease" || row.kind === "release" || row.kind === "change" || row.kind === "suggest" || row.kind === "concern" || row.kind === "command" || row.kind === "compaction" || row.kind === "disputed" || row.kind === "window" || row.kind === "started" || row.kind === "waiter" || row.kind === "linked" || row.kind === "ran" || row.kind === "check" || row.kind === "retro" || row.kind === "draft") return row;
    if (row.kind === "sent") {
      const { files, sketches, tags, ...rest } = row;
      return {
        ...rest,
        ...(files === undefined ? {} : { files: files.map(({ id, name, src }) => ({ id, name, ...(src === undefined ? {} : { src }) })) }),
        ...(sketches === undefined ? {} : { sketches }),
        ...(tags === undefined ? {} : { tags }),
      };
    }
    if (row.from.kind === "session") {
      return { id: row.id, at: row.at, kind: "message", from: "session", sender: { id: row.from.id, title: row.from.title }, text: row.text, ...(row.notice === undefined ? {} : { notice: row.notice }) };
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
      ...(row.delivery === undefined ? {} : { delivery: row.delivery }),
      ...(row.stableKey === undefined ? {} : { stableKey: row.stableKey }),
      ...(row.queued === true ? { queued: true as const } : {}),
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
