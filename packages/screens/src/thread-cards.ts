// What a Job's sketch and document cards send and where they stand in a thread, read off the ledger. Bridge's thread and the phone's
// both draw them, so the one reading is here.

import type { SessionThreadRow, SketchAnswer } from "@armada/components";
import type { AnswerDocument, AnswerSketch } from "@armada/protocol";
import type { Session, SessionAttachment } from "./draft/sessions";

type DocumentCard = Extract<
  SessionThreadRow,
  {
    kind: "document";
  }
>;


/**
 * What a press on a sketch's card sends: Approve is `choice: 0`, Send back its note. **`drawn` is the drawing the ledger holds
 * now**, which Fleet checks against the sketch as it stands; one the ledger never gave is sent empty, which Fleet refuses.
 */
export function sketchAnswerOf(
  session: Pick<Session, "id" | "attachments">,
  sketch: string,
  answer: SketchAnswer,
): AnswerSketch {
  const held = session.attachments.find((one) => one.kind === "sketch" && one.id === sketch);
  const drawn = held?.kind === "sketch" ? (held.drawn ?? "") : "";
  const base = {
    session_id: session.id,
    sketch,
    drawn,
  };
  return "text" in answer
    ? {
        ...base,
        how: "sent_back",
        words: answer.text,
      }
    : {
        ...base,
        how: "approved",
      };
}

/**
 * What a press on a document's Approve or Send back sends. **`revision` is the one the ledger holds now**, which Fleet checks against
 * the page as it stands; one the ledger never gave is sent empty, which Fleet refuses.
 */
export function documentAnswerOf(
  session: Pick<Session, "id" | "attachments">,
  page: string,
  answer: SketchAnswer,
): AnswerDocument {
  const held = session.attachments.find(
    (one) => one.kind === "artifact" && one.form === "document" && one.id === page,
  );
  const revision = held?.kind === "artifact" ? (held.revision ?? "") : "";
  const base = {
    session_id: session.id,
    document: `document:${page}`,
    revision,
  };
  return "text" in answer
    ? {
        ...base,
        how: "sent_back",
        words: answer.text,
      }
    : {
        ...base,
        how: "approved",
      };
}

/**
 * The card a Job's document is: its newest revision, read off the artifact and never off a write's tool row, so a page a script
 * changed has one too. It waits while that revision is asked and not settled. A page whose time the record does not carry has none.
 */
function documentCardOf(
  page: Extract<
    SessionAttachment,
    {
      kind: "artifact";
    }
  >,
): DocumentCard | undefined {
  if (page.form !== "document" || page.kept === undefined) return undefined;
  const revision =
    page.revision === undefined ? (page.revisions?.length ?? 0) : Number(page.revision);
  if (!Number.isInteger(revision) || revision < 1) return undefined;
  return {
    id: `document:${page.id}:r${String(revision)}`,
    at: page.kept,
    kind: "document",
    page: page.id,
    title: page.title,
    revision,
    ...(page.asked === true
      ? {
          asked: true as const,
        }
      : {}),
    ...(page.settled === undefined
      ? {}
      : {
          settled: page.settled,
        }),
  };
}

/**
 * Each document's card, where its revision was kept: before the first row later than that. **A row whose time does not read as one
 * is never later**, so the card follows it.
 */
export function withDocuments(
  rows: SessionThreadRow[],
  ledger: readonly SessionAttachment[],
): SessionThreadRow[] {
  const cards = ledger.flatMap((one) =>
    one.kind === "artifact" ? (documentCardOf(one) ?? []) : [],
  );
  if (cards.length === 0) return rows;
  const out = [
    ...rows,
  ];
  for (const card of [
    ...cards,
  ].sort((a, b) => Date.parse(a.at) - Date.parse(b.at))) {
    const kept = Date.parse(card.at);
    const later = out.findIndex((row) => Date.parse(row.at) > kept);
    out.splice(later < 0 ? out.length : later, 0, card);
  }
  return out;
}
