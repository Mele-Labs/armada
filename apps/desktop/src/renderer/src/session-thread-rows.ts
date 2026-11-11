// A Session's rows as its thread draws them, and a fork taken from words selected in one.

import { seedDraft } from "@armada/components";
import type { ForkPick, SessionThreadRow } from "@armada/components";
import type { Session, SessionAttachment, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { withDocuments } from "@armada/screens/src/thread-cards";
import { fileCallOf, keptFrom, sketchCallOf } from "@armada/screens/src/sessions-wire";
import { sceneOfSketch } from "./session-sketch";

export { documentAnswerOf, sketchAnswerOf } from "@armada/screens/src/thread-cards";

type SketchCard = Extract<
  SessionThreadRow,
  {
    kind: "sketch";
  }
>;

/**
 * What each Job's rows were last drawn as. **Another Job's update redraws the open thread**, and the store hands back the
 * same rows and ledger when this Job did not change (`SessionMemo`), so they are drawn once, not on every redraw.
 */
const drawn = new WeakMap<
  Session["rows"],
  {
    attachments: Session["attachments"];
    startFailed: Session["startFailed"];
    rows: SessionThreadRow[];
  }
>();

/** The rows a thread draws. The same list while the Job's rows, ledger and start are the same, so never change it in place. */
export function threadRowsOf(session: Session): SessionThreadRow[] {
  const held = drawn.get(session.rows);
  if (
    held !== undefined &&
    held.attachments === session.attachments &&
    held.startFailed === session.startFailed
  )
    return held.rows;
  const rows = drawnRowsOf(session);
  drawn.set(session.rows, {
    attachments: session.attachments,
    startFailed: session.startFailed,
    rows,
  });
  return rows;
}

function drawnRowsOf(session: Session): SessionThreadRow[] {
  const rows = threadRowsFrom(session.rows, session.attachments);
  // Fleet's line for a Job whose agent never started, after the message it was sent for.
  if (session.startFailed === undefined) return rows;
  return [
    ...rows,
    {
      id: "not-started",
      at: rows[rows.length - 1]?.at ?? session.lastTurnAt ?? new Date(0).toISOString(),
      kind: "not_started",
      why: `Did not start: ${session.startFailed.cause}`,
      log: session.startFailed.log,
    },
  ];
}

/** The rows up to and including the one with this id; all of them with no id. **An id no row has gives none**: a cut that cannot be found must not show what came after it. */
export function upTo<
  T extends {
    id: string;
  },
>(rows: readonly T[], at: string | undefined): readonly T[] {
  if (at === undefined) return rows;
  const end = rows.findIndex((one) => one.id === at);
  return end < 0 ? [] : rows.slice(0, end + 1);
}

/**
 * The card a `show_sketch` call is, where the thread carries the call as its tool row. **Only a call the ledger
 * holds a sketch for draws one**: Fleet keeps the sketch it accepted under the call's title, so a call it refused
 * is no card, and neither is one whose scene would not draw.
 */
function sketchCardOf(
  row: {
    id: string;
    at: string;
    text: string;
  },
  ledger: readonly SessionAttachment[],
): SketchCard | undefined {
  const call = sketchCallOf(row.text);
  if (call === undefined) return undefined;
  const sketch = `sketch:${call.title}`;
  const held = ledger.find((one) => one.kind === "sketch" && one.id === sketch);
  if (held === undefined || held.kind !== "sketch") return undefined;
  // The call is the agent's words, without the `kept` Fleet wrote on the sketch it kept.
  return {
    id: row.id,
    at: row.at,
    kind: "sketch",
    sketch,
    title: call.title,
    drawing: keptFrom(call.scene, sceneOfSketch(held.drawing)),
    ...(call.against === undefined
      ? {}
      : {
          against: call.against,
        }),
  };
}

/**
 * The row a `show_file` call is, where the ledger holds the file. **Opened by the ledger's own path**, the canonical one Fleet
 * kept: the call's text is the path as the agent gave it, which may be relative, and main opens only a path the ledger names.
 * A call whose file the ledger does not hold stays the plain tool row it was.
 */
function fileRowOf(
  row: {
    id: string;
    at: string;
    text: string;
  },
  ledger: readonly SessionAttachment[],
):
  | Extract<
      SessionThreadRow,
      {
        kind: "file";
      }
    >
  | undefined {
  const call = fileCallOf(row.text);
  if (call === undefined) return undefined;
  const given = call.path.replace(/^\.\//, "");
  const held = ledger.find(
    (one) =>
      one.kind === "artifact" &&
      one.form === "file" &&
      (one.id === call.path || one.id.endsWith(`/${given}`)),
  );
  return held?.kind === "artifact"
    ? {
        id: row.id,
        at: row.at,
        kind: "file",
        path: held.id,
        name: held.title,
      }
    : undefined;
}

/**
 * A card is replaced where a later card holds the same sketch. What it was drawn against is the latest card before it
 * with the title it names, as it was drawn then, else the sketch the ledger keeps the earlier scene of.
 */
function withSketchState(
  rows: SessionThreadRow[],
  ledger: readonly SessionAttachment[],
): SessionThreadRow[] {
  const last = new Map(
    rows.flatMap((row) =>
      row.kind === "sketch"
        ? [
            [
              row.sketch,
              row.id,
            ] as const,
          ]
        : [],
    ),
  );
  return rows.map((row, at): SessionThreadRow => {
    if (row.kind !== "sketch") return row;
    const before =
      row.against === undefined
        ? undefined
        : rows
            .slice(0, at)
            .reverse()
            .find((one) => one.kind === "sketch" && one.title === row.against);
    const held = ledger.find((one) => one.kind === "sketch" && one.id === row.sketch);
    const kept =
      held?.kind === "sketch" && held.against !== undefined && held.against.title === row.against
        ? sceneOfSketch(held.against.drawing)
        : undefined;
    const earlier = before?.kind === "sketch" ? before.drawing : kept;
    const latest = last.get(row.sketch) === row.id;
    // What he was asked, and how he answered, belong to the sketch as it stands: the latest card holds them.
    const answer =
      !latest || held?.kind !== "sketch"
        ? {}
        : {
            ...(held.asked === true
              ? {
                  asked: true as const,
                }
              : {}),
            ...(held.withdrawn === true
              ? {
                  withdrawn: true as const,
                }
              : {}),
            ...(held.settled === undefined
              ? {}
              : {
                  settled: held.settled,
                }),
          };
    return {
      ...row,
      ...(earlier === undefined
        ? {}
        : {
            againstScene: earlier,
          }),
      ...(latest
        ? {}
        : {
            replaced: true as const,
          }),
      ...answer,
    };
  });
}

export function threadRowsFrom(
  rows: Session["rows"],
  ledger: readonly SessionAttachment[] = [],
): SessionThreadRow[] {
  return openedBeforeBareQuestions(
    withDocuments(withSketchState(rowsDrawn(rows, ledger), ledger), ledger),
  );
}

/**
 * The reasoning just before a question opens where the agent wrote nothing to the person since their message. **They
 * read only what the agent writes**, so a Job that works its answer out in its reasoning and then asks leaves them the
 * question alone (Job 20b4e00a, 10 Nov 2026). Every other reasoning row stays folded.
 */
function openedBeforeBareQuestions(rows: SessionThreadRow[]): SessionThreadRow[] {
  let owed = false;
  return rows.map((row, index) => {
    if (row.kind === "message" && row.from === "you") owed = true;
    if (row.kind === "message" && row.from === "agent") owed = false;
    const next = rows[index + 1];
    return row.kind === "reasoning" &&
      owed &&
      next?.kind === "tool" &&
      next.text.startsWith("AskUserQuestion")
      ? {
          ...row,
          open: true as const,
        }
      : row;
  });
}

function rowsDrawn(
  rows: Session["rows"],
  ledger: readonly SessionAttachment[],
): SessionThreadRow[] {
  // A browser reminder is the Job browser's own card in the thread (`job-browser/JobBrowserPane.tsx`), not a row.
  return rows
    .flatMap((row) =>
      row.kind === "browser_reminder"
        ? []
        : [
            row,
          ],
    )
    .map((row): SessionThreadRow => {
      if (row.kind === "tool") return sketchCardOf(row, ledger) ?? fileRowOf(row, ledger) ?? row;
      if (row.kind === "requested" || row.kind === "saved") return row;
      if (
        row.kind === "lease" ||
        row.kind === "release" ||
        row.kind === "change" ||
        row.kind === "suggest" ||
        row.kind === "command" ||
        row.kind === "compaction" ||
        row.kind === "reasoning" ||
        row.kind === "disputed" ||
        row.kind === "window" ||
        row.kind === "started" ||
        row.kind === "waiter" ||
        row.kind === "linked" ||
        row.kind === "operator" ||
        row.kind === "ran" ||
        row.kind === "check" ||
        row.kind === "retro" ||
        row.kind === "draft"
      )
        return row;
      if (row.kind === "sent") {
        const { files, sketches, tags, ...rest } = row;
        return {
          ...rest,
          ...(files === undefined
            ? {}
            : {
                files: files.map(({ id, name, src }) => ({
                  id,
                  name,
                  ...(src === undefined
                    ? {}
                    : {
                        src,
                      }),
                })),
              }),
          ...(sketches === undefined
            ? {}
            : {
                sketches,
              }),
          ...(tags === undefined
            ? {}
            : {
                tags,
              }),
        };
      }
      if (row.from.kind === "session") {
        return {
          id: row.id,
          at: row.at,
          kind: "message",
          from: "session",
          sender: {
            id: row.from.id,
            title: row.from.title,
          },
          text: row.text,
          ...(row.notice === undefined
            ? {}
            : {
                notice: row.notice,
              }),
        };
      }
      return {
        id: row.id,
        at: row.at,
        kind: "message",
        from: row.from.kind,
        text: row.text,
        ...(row.files === undefined
          ? {}
          : {
              files: row.files,
            }),
        ...(row.sketches === undefined
          ? {}
          : {
              sketches: row.sketches,
            }),
        ...(row.tags === undefined
          ? {}
          : {
              tags: row.tags,
            }),
        ...(row.delivery === undefined
          ? {}
          : {
              delivery: row.delivery,
            }),
        ...(row.stableKey === undefined
          ? {}
          : {
              stableKey: row.stableKey,
            }),
        ...(row.queued === true
          ? {
              queued: true as const,
            }
          : {}),
      };
    });
}

/** Fork at the message the words were selected in. The words are in the new Session's box before it opens, with the caret behind them. */
export async function forkAtPick(
  forkAt: NonNullable<SessionsDraft["forkAt"]>,
  id: string,
  { rowId, quote }: ForkPick,
  focus: (id: string) => void,
): Promise<string | undefined> {
  const forked = await forkAt(id, rowId);
  if (forked !== undefined) {
    seedDraft(forked, quote);
    focus(forked);
  }
  return forked;
}
