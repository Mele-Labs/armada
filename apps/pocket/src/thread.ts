// A Job's thread as the phone draws it: Fleet's `SessionThread` (from `/api/sessions/one`) read with
// Bridge's own wire mapping (`@armada/screens`), then laid on `SessionThread`'s rows the way Bridge's
// `session-thread-rows.ts` does, with one difference: a walk window is a row saying it opens on the Mac.

import { AppWindow } from "lucide-react";
import type {
  AskOffer,
  AskedQuestion,
  PlanApproval,
  Scene,
  SessionThreadRow,
  SketchSettled,
} from "@armada/components";
import type {
  SessionThread as WireThread,
  WaitingItem,
  WaitingOption,
  WaitingSource,
} from "@armada/protocol";
import { helmOfferedOf, reachOfferedOf } from "@armada/screens/src/copy";
import {
  planAsked,
  type Session,
  type SessionAttachment,
  type SessionSketch,
} from "@armada/screens/src/draft/sessions";
import {
  fileCallOf,
  keptFrom,
  sessionOfRecord,
  sketchCallOf,
} from "@armada/screens/src/sessions-wire";
import { withDocuments } from "@armada/screens/src/thread-cards";

const NOTHING_BESIDE = {
  picture: () => undefined,
  sketches: [],
  pending: [],
};

export const sessionOfThread = (wire: WireThread): Session =>
  sessionOfRecord(wire.session, wire.rows, NOTHING_BESIDE);

export function sceneOfSketch(drawing: SessionSketch | Scene): Scene {
  if ("nodes" in drawing) return drawing;
  return {
    nodes: drawing.boxes.map((one) => ({
      id: one.id,
      kind: "box",
      x: one.x,
      y: one.y,
      body: one.body,
    })),
    edges: drawing.lines.map((one) => ({
      id: one.id,
      from: one.from,
      to: one.to,
    })),
    ...(drawing.strokes === undefined
      ? {}
      : {
          strokes: drawing.strokes,
        }),
  };
}

type Ledger = readonly SessionAttachment[];
type Row = {
  id: string;
  at: string;
  text: string;
};

/** A `show_sketch` call the ledger holds a sketch for, as its card. */
function sketchCardOf(row: Row, ledger: Ledger): SessionThreadRow | undefined {
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

/** A `show_file` call whose file the ledger holds, opened by the ledger's own path. */
function fileRowOf(row: Row, ledger: Ledger): SessionThreadRow | undefined {
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

/** A card is replaced where a later card holds the same sketch, and is drawn against the card it names. */
function withSketchState(rows: SessionThreadRow[], ledger: Ledger): SessionThreadRow[] {
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

export const OPEN_ON_MAC = "Open on your Mac";

export function rowsOf(session: Session): SessionThreadRow[] {
  const ledger = session.attachments;
  // A browser reminder offers the Mac's Job browser, which the phone has none of. A draft still open is its card at the
  // thread's end (`draftsOf`), so it is drawn there alone; once settled, the row says how it ended.
  const open = new Set(draftsOf(session).map((one) => one.id));
  const drawn = session.rows
    .flatMap((row) =>
      row.kind === "browser_reminder" || (row.kind === "draft" && open.has(row.id))
        ? []
        : [
            row,
          ],
    )
    .map((row): SessionThreadRow => {
      if (row.kind === "tool") return sketchCardOf(row, ledger) ?? fileRowOf(row, ledger) ?? row;
      // A walk is the Mac's: the phone says where it is and nothing more.
      if (row.kind === "window")
        return {
          id: row.id,
          at: row.at,
          kind: "act",
          icon: AppWindow,
          said: row.title,
          text: OPEN_ON_MAC,
        };
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
      if (row.kind !== "message") return row;
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
      };
    });
  return withDocuments(withSketchState(drawn, ledger), ledger);
}

export type Asked = {
  command: string;
  offers?: readonly AskOffer[];
  questions?: readonly AskedQuestion[];
  reaches?: string;
  reachesTo?: "read" | "write" | "run";
};

/** What the agent is held on, worded as Bridge words it. */
export function askedOf(session: Session): Asked | undefined {
  const asked = session.asked;
  if (asked === undefined) return undefined;
  return {
    command: asked.command,
    ...(asked.questions === undefined
      ? {}
      : {
          questions: asked.questions,
        }),
    ...(asked.offers === undefined
      ? {}
      : {
          offers: (asked.reaches === undefined ? helmOfferedOf : reachOfferedOf)(asked.offers).map(
            (one) => ({
              id: one.offer,
              label: one.label,
              means: one.means,
            }),
          ),
        }),
    ...(asked.reaches === undefined
      ? {}
      : {
          reaches: asked.reaches,
        }),
    ...(asked.reachesTo === undefined
      ? {}
      : {
          reachesTo: asked.reachesTo,
        }),
  };
}

/** The plan, while Fleet puts it to the owner or once approved. */
export function approvalOf(session: Session, onApprove: () => void): PlanApproval | undefined {
  const plan = session.work?.plan;
  if (plan === undefined || !(plan.approved || planAsked(session))) return undefined;
  return {
    ...(plan.version === undefined
      ? {}
      : {
          version: plan.version,
        }),
    groups: plan.groups.map((one) => ({
      id: one.id,
      agent: one.agent,
      title: one.title,
      tasks: one.tasks.map((task) => ({
        id: task.id,
        text: task.text,
      })),
    })),
    approved: plan.approved,
    ...(plan.sketch === undefined
      ? {}
      : {
          sketch: {
            id: plan.sketch.id,
            title: plan.sketch.title,
            scene: plan.sketch.scene,
          },
        }),
    ...(plan.after === undefined
      ? {}
      : {
          after: plan.after,
        }),
    onApprove,
  };
}

/** Fleet's items that a choice settles from here, which take no words. */
const CHOICE_ONLY: readonly WaitingSource[] = [
  "delivery",
  "slot_choice",
  "task_done",
  "rule_act",
  "browser_ask",
];
/** The sources answered beside the thread. The rest are answered from their own cards, or on the Mac. */
const ASKS: readonly WaitingSource[] = [
  "agent",
  "watch_not_ours",
  "words_address",
  ...CHOICE_ONLY,
];

/**
 * What the owner is asked beside the thread: an item with its choices, answered on the Mac's `waiting/answer`.
 * An item whose act is a command is a Run card (`runsOf`) instead.
 */
export const asksOf = (session: Session): readonly WaitingItem[] =>
  session.dead === undefined
    ? (session.waitingFor ?? []).filter(
        (one) => ASKS.includes(one.source) && one.act?.kind !== "run",
      )
    : [];

export const takesWords = (item: WaitingItem): boolean => !CHOICE_ONLY.includes(item.source);

/**
 * The options to press, each with its index on the wire. An address's `Other` is the words box, since Fleet refuses it
 * without words, and a slot being made asks for nothing.
 */
export const choicesOf = (
  item: WaitingItem,
): readonly (WaitingOption & {
  choice: number;
})[] =>
  item.creating_since === undefined
    ? (item.options ?? []).flatMap((option, choice) =>
        item.source === "words_address" && option.label === "Other"
          ? []
          : [
              {
                ...option,
                choice,
              },
            ],
      )
    : [];

export type RunItem = {
  item: WaitingItem;
  command: string /** A run of the command ended after the item was put: its exit code, absent where it was stopped. */;
  ran?: {
    code?: number;
  };
};

/**
 * The items whose act is a command to run on the Mac, each a Run card beside the thread. Fleet writes a `ran` row when a run
 * ends and the agent clears the item itself, so an item a newer `ran` row answers is drawn as ran until it goes.
 */
export function runsOf(session: Session, wire: WireThread): readonly RunItem[] {
  if (session.dead !== undefined) return [];
  return (session.waitingFor ?? []).flatMap((one) => {
    if (one.act?.kind !== "run") return [];
    const command = one.act.target;
    const since = Date.parse(one.since);
    const ran = [
      ...(wire.rows ?? []),
    ]
      .reverse()
      .find(
        (row) =>
          row.kind === "ran" && row.command.trim() === command.trim() && Date.parse(row.at) > since,
      );
    return [
      {
        item: one,
        command,
        ...(ran?.kind === "ran"
          ? {
              ran:
                ran.code === undefined
                  ? {}
                  : {
                      code: ran.code,
                    },
            }
          : {}),
      },
    ];
  });
}

/**
 * What Fleet puts on a Job's list that is pressed and not answered. Press Start on a Job this one started opens that Job, which opens
 * as its start card; Fleet refuses to answer or dismiss it. A call a Fleet rule raised is dismissed, and opens the Job it names.
 */
export type PressItem = {
  item: WaitingItem;
  /** The Job a press opens. */
  opens?: string;
  dismissible: boolean;
};

export function pressesOf(session: Session): readonly PressItem[] {
  if (session.dead !== undefined) return [];
  return (session.waitingFor ?? []).flatMap((one): PressItem[] => {
    if (one.source !== "held_child" && one.source !== "fleet_call") return [];
    return [
      {
        item: one,
        ...(one.act?.kind === "message"
          ? {
              opens: one.act.target,
            }
          : {}),
        dismissible: one.source === "fleet_call",
      },
    ];
  });
}

/** The walks and the questions in a walk the Job waits on: pages the phone cannot show, each said to open on the Mac. */
export const onMacOf = (session: Session): readonly WaitingItem[] =>
  session.dead === undefined
    ? (session.waitingFor ?? []).filter((one) => one.source === "walk" || one.source === "window_ask")
    : [];

export type OpenDraft = {
  id: string;
  title: string;
  text: string;
  before?: string;
  /** A proposed workflow, which Fleet also lets him drop: its item's second option. */
  proposed?: true;
};

/**
 * The drafts the agent proposed that wait on the owner, each settled on the Mac as the item `draft:<row id>`. **A workflow change is
 * one**: Fleet lists its proposal only while its change is ready, so a card is drawn only where the item is held, never off the row.
 */
export function draftsOf(session: Session): readonly OpenDraft[] {
  if (session.dead !== undefined) return [];
  const waiting = new Map((session.waitingFor ?? []).map((one) => [
    one.id,
    one,
  ]));
  return session.rows.flatMap((row) =>
    row.kind === "draft" &&
    row.settled === undefined &&
    waiting.has(`draft:${row.id}`)
      ? [
          {
            id: row.id,
            title: row.title,
            text: row.text,
            ...(row.before === undefined
              ? {}
              : {
                  before: row.before,
                }),
            ...((waiting.get(`draft:${row.id}`)?.options?.length ?? 0) > 1
              ? {
                  proposed: true as const,
                }
              : {}),
          },
        ]
      : [],
  );
}

export type JobLink = {
  id: string;
  title: string;
};

/** The Job this one was started from, and the Jobs it started or follows, from its record and its thread's rows. */
export function linksOf(wire: WireThread): {
  from?: JobLink;
  started: readonly JobLink[];
} {
  const by = wire.session.started?.by;
  const seen = new Set<string>(
    by === undefined
      ? []
      : [
          by.id,
          wire.session.id,
        ],
  );
  const started: JobLink[] = [];
  for (const row of wire.rows ?? []) {
    if (
      row.kind !== "started" &&
      row.kind !== "linked" &&
      !(row.kind === "waiter" && row.side === undefined)
    )
      continue;
    if (seen.has(row.session.id)) continue;
    seen.add(row.session.id);
    started.push({
      id: row.session.id,
      title: row.session.title,
    });
  }
  return {
    ...(by === undefined
      ? {}
      : {
          from: {
            id: by.id,
            title: by.title,
          },
        }),
    started,
  };
}

/** The sketch the Job showed, by its ledger id, as the viewer draws it. */
export function sketchOf(
  session: Session,
  id: string,
):
  | {
      title: string;
      scene: Scene;
      against?: Scene;
    }
  | undefined {
  const held = session.attachments.find((one) => one.kind === "sketch" && one.id === id);
  if (held?.kind !== "sketch") return undefined;
  return {
    title: held.title,
    scene: sceneOfSketch(held.drawing),
    ...(held.against === undefined
      ? {}
      : {
          against: sceneOfSketch(held.against.drawing),
        }),
  };
}

/** A page the Job wrote as its document, by the ledger id the thread's card names. `waits` is its newest revision put to him and not answered. */
export function documentOf(
  session: Session,
  page: string,
):
  | {
      title: string;
      revisions: readonly string[];
      revision: number;
      waits: boolean;
      settled?: SketchSettled;
      keptStyles?: Readonly<Record<string, string>>;
    }
  | undefined {
  const held = session.attachments.find(
    (one) => one.kind === "artifact" && one.form === "document" && one.id === page,
  );
  if (held?.kind !== "artifact" || held.revisions === undefined || held.revisions.length === 0)
    return undefined;
  return {
    title: held.title,
    revisions: held.revisions,
    revision: held.revision === undefined ? held.revisions.length : Number(held.revision),
    waits: held.asked === true && held.settled === undefined,
    ...(held.settled === undefined
      ? {}
      : {
          settled: held.settled,
        }),
    ...(held.keptStyles === undefined
      ? {}
      : {
          keptStyles: held.keptStyles,
        }),
  };
}
