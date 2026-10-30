// A Session's rows as its thread draws them, and a fork taken from words selected in one.

import { seedDraft } from "@armada/components";
import type { ForkPick, SessionThreadRow, SketchAnswer } from "@armada/components";
import type { AnswerDocument, AnswerSketch } from "@armada/protocol";
import type { Session, SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";
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

/**
 * What a press on a document's Approve or Send back sends. **`revision` is the one the ledger holds now**, which Fleet checks against
 * the page as it stands; one the ledger never gave is sent empty, which Fleet refuses.
 */
export function documentAnswerOf(session: Pick<Session, "id" | "attachments">, page: string, answer: SketchAnswer): AnswerDocument {
  const held = session.attachments.find((one) => one.kind === "artifact" && one.form === "document" && one.id === page);
  const revision = held?.kind === "artifact" ? (held.revision ?? "") : "";
  const base = { session_id: session.id, document: `document:${page}`, revision };
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

/**
 * The card a Job's document is: its newest revision, read off the artifact and never off a write's tool row, so a page a script
 * changed has one too. It waits while that revision is asked and not settled. A page whose time the record does not carry has none.
 */
function documentCardOf(page: Extract<SessionAttachment, { kind: "artifact" }>): DocumentCard | undefined {
  if (page.form !== "document" || page.kept === undefined) return undefined;
  const revision = page.revision === undefined ? (page.revisions?.length ?? 0) : Number(page.revision);
  if (!Number.isInteger(revision) || revision < 1) return undefined;
  return {
    id: `document:${page.id}:r${String(revision)}`,
    at: page.kept,
    kind: "document",
    page: page.id,
    title: page.title,
    revision,
    ...(page.asked === true ? { asked: true as const } : {}),
    ...(page.settled === undefined ? {} : { settled: page.settled }),
  };
}

/**
 * Each document's card, where its revision was kept: before the first row later than that. **A row whose time does not read as one
 * is never later**, so the card follows it.
 */
function withDocuments(rows: SessionThreadRow[], ledger: readonly SessionAttachment[]): SessionThreadRow[] {
  const cards = ledger.flatMap((one) => (one.kind === "artifact" ? (documentCardOf(one) ?? []) : []));
  if (cards.length === 0) return rows;
  const out = [...rows];
  for (const card of [...cards].sort((a, b) => Date.parse(a.at) - Date.parse(b.at))) {
    const kept = Date.parse(card.at);
    const later = out.findIndex((row) => Date.parse(row.at) > kept);
    out.splice(later < 0 ? out.length : later, 0, card);
  }
  return out;
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
  return withDocuments(withSketchState(rowsDrawn(rows, ledger), ledger), ledger);
}

function rowsDrawn(rows: Session["rows"], ledger: readonly SessionAttachment[]): SessionThreadRow[] {
  // A browser reminder is the Job browser's own card in the thread (`job-browser/JobBrowserPane.tsx`), not a row.
  return rows.flatMap((row) => (row.kind === "browser_reminder" ? [] : [row])).map((row): SessionThreadRow => {
    if (row.kind === "tool") return sketchCardOf(row, ledger) ?? fileRowOf(row, ledger) ?? row;
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
