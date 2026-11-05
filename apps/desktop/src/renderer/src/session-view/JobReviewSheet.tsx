// What a Job reads before it goes on, on one sheet: a plan, a sketch, a draft, a document and a walk. The Review row on the Needs you band or `⇧⌘Y` opens it: the items oldest first in a list, the selected one
// beside it with its answers, an answer bringing the next open item up and the last closing the sheet. **Nothing here is a second path to Fleet**: each answer calls the
// handler the same item's card or row calls (`sessions.tsx` hands them in), so an answer given here settles the card in the thread and the reverse.
// What he answers rather than reads, a question, a command, a pull request, a message and Fleet's own cards, stays on the Needs you card. The band's Review glyphs are this list's, so the two agree.

import { useRef, useState, type ReactNode } from "react";
import { X } from "lucide-react";
import {
  DraftRow,
  REVIEW_MARKS,
  ReviewSheet,
  waitingMark,
  type DraftThreadRow,
  type LedgerEntry,
  type ReviewItem,
  type ReviewMark,
} from "@armada/components";
import { useAtFloor } from "@armada/shell";
import {
  attachmentsOf,
  owesOf,
  type Session,
  type SessionsDraft,
} from "@armada/screens/src/draft/sessions";

import { dismissWaiting } from "../cockpit/dismissed";
import { sketchAnswerOf, documentAnswerOf } from "../session-thread-rows";
import { DocumentBody } from "./DocumentLayer";
import { OverTheApp } from "./OverTheApp";
import { SketchBody } from "./SketchBody";
import { WalkPane } from "../job-walk/WalkPane";
import { walksOf } from "../job-walk/walks";

/** What the sheet is told of the Job: the handlers its cards and rows already call, and the bodies only `sessions.tsx` can build. */
export type ReviewParts = {
  session: Session;
  draft: SessionsDraft;
  title: string;
  /** What waits as the ledger draws it, by key: a row for each waiting item, which carries the item's `since` and what it opens. */
  rows: readonly LedgerEntry[];
  /** The plan put to him: its body, and the thread card's two acts on it. */
  plan?: {
    body: ReactNode;
    approve: () => void;
    sendBack: (note: string) => void;
  };
  /** The thread's draft cards, by the row each is. */
  drafts: ReadonlyMap<string, DraftThreadRow>;
};

/** One thing waiting, built. `since` orders the list, oldest first; `said` is the band's tooltip. */
export type Waiting = {
  id: string;
  since: number;
  said: string;
  mark: ReviewMark;
  item: Omit<ReviewItem, "state" | "said">;
  /** What the primary does, and what a row it answered says. */
  primary?: {
    run: () => void;
    said: string;
  };
  sendBack?: (note: string) => void;
};

const sinceOf = (at: string | undefined): number => {
  const ms = at === undefined ? Number.NaN : Date.parse(at);
  return Number.isNaN(ms) ? 0 : ms;
};

/**
 * Every item the Job has to read, built: the item's body, the answers it offers, and what each calls. **Oldest first.** A row the sheet has no body for is
 * not built, and stays with the card.
 */
export function waitingOf(parts: ReviewParts): Waiting[] {
  const { session, draft } = parts;
  const owed = new Map(
    owesOf(session).map((one) => [
      one.id,
      one,
    ]),
  );
  const made: Waiting[] = [];
  for (const row of parts.rows) {
    const one = owed.get(row.key.replace(/^waiting/, ""));
    const since = sinceOf(one?.since);
    const named = (
      mark: ReviewMark,
      body: ReactNode,
      answers?: {
        verb: string;
        sendBack: boolean;
        primary: Waiting["primary"];
        sendBackTo?: Waiting["sendBack"];
      },
    ): Waiting => ({
      id: row.key,
      since,
      said: row.name,
      mark,
      item: {
        id: row.key,
        mark,
        name: row.name,
        body,
        ...(answers === undefined
          ? {}
          : {
              answer: {
                verb: answers.verb,
                sendBack: answers.sendBack,
              },
            }),
        ...menuOf(row),
      },
      ...(answers?.primary === undefined
        ? {}
        : {
            primary: answers.primary,
          }),
      ...(answers?.sendBackTo === undefined
        ? {}
        : {
            sendBack: answers.sendBackTo,
          }),
    });
    /** Approve with Send back: the three kinds put to him with a card of their own. */
    const approval = (verb: string, approve: () => void, back: (note: string) => void) => ({
      verb,
      sendBack: true,
      primary: {
        run: approve,
        said: "Approved",
      },
      sendBackTo: back,
    });
    if (one?.source === "plan" && parts.plan !== undefined) {
      const { plan } = parts;
      made.push(
        named(REVIEW_MARKS.plan, plan.body, approval("Approve", plan.approve, plan.sendBack)),
      );
      continue;
    }
    const sketch =
      one?.source === "sketch"
        ? attachmentsOf(session, "sketch").find((it) => it.id === one.id)
        : undefined;
    if (sketch !== undefined && draft.answerSketch !== undefined) {
      const send = (
        given:
          | {
              choice: 0;
            }
          | {
              text: string;
            },
      ) => void draft.answerSketch?.(sketchAnswerOf(session, sketch.id, given));
      made.push(
        named(
          REVIEW_MARKS.sketch,
          <div className="armada-job-review__fill">
            <SketchBody sessionId={session.id} one={sketch} draft={draft} />
          </div>,
          approval(
            "Approve",
            () =>
              send({
                choice: 0,
              }),
            (text) =>
              send({
                text,
              }),
          ),
        ),
      );
      continue;
    }
    const page = one?.source === "document" ? one.id.replace(/^document:/, "") : undefined;
    const document =
      page === undefined
        ? undefined
        : attachmentsOf(session, "artifact").find((it) => it.form === "document" && it.id === page);
    if (document !== undefined && draft.answerDocument !== undefined) {
      const send = (
        given:
          | {
              choice: 0;
            }
          | {
              text: string;
            },
      ) => void draft.answerDocument?.(documentAnswerOf(session, document.id, given));
      made.push(
        named(
          REVIEW_MARKS.document,
          <div className="armada-job-review__fill">
            <DocumentBody session={session} page={document.id} />
          </div>,
          approval(
            "Approve",
            () =>
              send({
                choice: 0,
              }),
            (text) =>
              send({
                text,
              }),
          ),
        ),
      );
      continue;
    }
    const card =
      one?.source === "draft" ? parts.drafts.get(one.id.replace(/^draft:/, "")) : undefined;
    if (one !== undefined && card !== undefined && draft.answerWaiting !== undefined) {
      // The head holds Approve and Send back, named for the draft, as it does for every item. The card keeps what is its own: the text, the text it replaces, Edit and Withdraw.
      const answer = (
        given:
          | {
              choice: 0;
            }
          | {
              text: string;
            },
      ) => void draft.answerWaiting?.(session.id, one.id, given);
      // A rule or a proposed workflow is sent as it stands: it has no note to send back with.
      const asNoted = card.rule === undefined && card.workflow === undefined;
      made.push(
        named(
          REVIEW_MARKS.draft,
          <ul className="armada-job-review__fill">
            <DraftRow
              row={card}
              keep={`${session.id}:${one.id}`}
              headed
              onAnswer={(rowId, given) =>
                void draft.answerWaiting?.(session.id, `draft:${rowId}`, given)
              }
              onWithdraw={(rowId, reason) =>
                void dismissWaiting(session.id, `draft:${rowId}`, reason)
              }
            />
          </ul>,
          {
            verb: card.rule === undefined ? "Approve" : "Send",
            sendBack: asNoted,
            primary: {
              run: () =>
                answer({
                  choice: 0,
                }),
              said: card.rule === undefined ? "Approved" : "Sent",
            },
            ...(asNoted
              ? {
                  sendBackTo: (text: string) =>
                    answer({
                      text,
                    }),
                }
              : {}),
          },
        ),
      );
      continue;
    }
    const target = one?.source === "walk" && one.act?.kind === "walk" ? one.act.target : undefined;
    // A walk's pane draws the page and its bar, and the bar's own Approve is the only one.
    if (
      target !== undefined &&
      row.options === undefined &&
      walksOf(session).some((it) => it.id === target)
    ) {
      made.push(
        named(
          waitingMark(row),
          <div className="armada-job-review__fill armada-job-review__bleed">
            <WalkPane jobId={session.id} walkId={target} front />
          </div>,
        ),
      );
      continue;
    }
  }
  return made.sort((a, b) => (a.since === b.since ? 0 : a.since < b.since ? -1 : 1));
}

/** A row's right-press menu: what puts it away. */
function menuOf(row: LedgerEntry): Pick<ReviewItem, "menu"> {
  if (row.onDismiss === undefined) return {};
  return {
    menu: {
      entries: [
        {
          kind: "item",
          id: "dismiss",
          label: row.dismissSaid ?? "Dismiss",
          icon: X,
        },
      ],
      onSelect: () => row.onDismiss?.(),
    },
  };
}

/**
 * The rows the Needs you card keeps: those the sheet built nothing for. A plan, a sketch and a document are never listed on the card, which has no row for them
 * on `origin/main` either: the thread's card shows them whole.
 */
export function cardRowsOf(
  session: Session,
  rows: readonly LedgerEntry[],
  built: readonly Waiting[],
): LedgerEntry[] {
  const taken = new Set(built.map((one) => one.id));
  const read = new Map(
    owesOf(session).map((one) => [
      `waiting${one.id}`,
      one.source,
    ]),
  );
  return rows.filter((row) => {
    const source = read.get(row.key);
    return !taken.has(row.key) && source !== "plan" && source !== "sketch" && source !== "document";
  });
}

type Settled = {
  state: "answered" | "sent_back";
  said?: string;
};

/**
 * The sheet, told by the Job's view whether it is open. **An item that goes from the list while it is open was answered**, from here or
 * from its card, and stays in its place muted until the sheet closes: the next open item comes up, and the last answer closes the sheet.
 */
export function JobReviewSheet({
  parts,
  waiting,
  open,
  onClose,
}: {
  parts: ReviewParts;
  waiting: readonly Waiting[];
  open: boolean;
  onClose: () => void;
}) {
  const floor = useAtFloor();
  const [settled, setSettled] = useState<Readonly<Record<string, Settled>>>({});
  const seen = useRef(new Map<string, Waiting>());
  const [wasOpen, setWasOpen] = useState(open);
  if (wasOpen !== open) {
    setWasOpen(open);
    if (!open) {
      seen.current = new Map();
      setSettled({});
    }
  }
  // What went since the sheet opened stays where it was, with the last body it drew.
  const live = new Set(waiting.map((one) => one.id));
  if (open) for (const one of waiting) seen.current.set(one.id, one);
  const list = open
    ? [
        ...seen.current.values(),
      ].sort((a, b) => (a.since === b.since ? 0 : a.since < b.since ? -1 : 1))
    : [
        ...waiting,
      ];

  const settle = (id: string, how: Settled) =>
    setSettled((was) =>
      was[id] === undefined
        ? {
            ...was,
            [id]: how,
          }
        : was,
    );
  const items = list.map((one): ReviewItem => {
    const how = settled[one.id];
    const gone = open && !live.has(one.id);
    const state = how?.state ?? (gone ? "answered" : "open");
    const menu = one.item.menu;
    return {
      ...one.item,
      // A row put away from its menu says so, rather than Answered.
      ...(menu === undefined
        ? {}
        : {
            menu: {
              entries: menu.entries,
              onSelect: (entry: string) => {
                menu.onSelect(entry);
                if (entry === "dismiss")
                  settle(one.id, {
                    state: "answered",
                    said: "Dismissed",
                  });
              },
            },
          }),
      state,
      ...(how?.said === undefined
        ? {}
        : {
            said: how.said,
          }),
    };
  });
  const by = new Map(
    list.map((one) => [
      one.id,
      one,
    ]),
  );
  return (
    <OverTheApp open={open}>
      <ReviewSheet
        open={open}
        job={parts.title}
        items={items}
        keep={`review:${parts.session.id}`}
        floor={floor}
        onPrimary={(id) => {
          const one = by.get(id);
          if (one?.primary === undefined) return;
          one.primary.run();
          settle(id, {
            state: "answered",
            said: one.primary.said,
          });
        }}
        onSendBack={(id, note) => {
          by.get(id)?.sendBack?.(note);
          settle(id, {
            state: "sent_back",
          });
        }}
        onClose={onClose}
      />
    </OverTheApp>
  );
}
