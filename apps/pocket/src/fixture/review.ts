// The fixture's To review: every live Job's `waiting_for`, read as Fleet's `list_to_review` reads it
// (`crates/fleet/src/session_host/to_review.rs`), so an answer the fixture takes off a Job's list leaves this one too.
// The rows are a short reading of `needs_you_of`: enough act and dismiss for the phone to pick each item's form.

import type {
  NeedsYouAct,
  NeedsYouDismiss,
  NeedsYouGlyphKind,
  SessionThread,
  ToReview,
  ToReviewAnswer,
  ToReviewGroup,
  ToReviewHead,
  ToReviewItem,
  ToReviewKind,
  WaitingItem,
} from "../wire";

import type { FixtureMac } from "./wire";

/** The sub project and the feature each fixture Job is filed under, so the list shows every kind of head. */
const SUB_PROJECT = {
  id: "sp-phone",
  name: "Phone review",
  goal: "Answer everything that needs me from the phone",
};
const POCKET = {
  project: "armada",
  slug: "pocket",
  title: "Pocket",
  area: "bridge",
  area_title: "Bridge",
  area_place: 1,
  place: 3,
};
const FILED: Record<string, "sub_project" | "feature"> = {
  f1: "sub_project",
  rv: "sub_project",
  g7: "feature",
};

/** Words besides the options, where Fleet takes them. */
const CHOICE_ONLY: readonly string[] = [
  "permission",
  "slot_choice",
  "delivery",
  "first_message",
  "held_child",
  "browser_ask",
];

/** `pressed` in `needs_you.rs`, short: what the row's press does before it is known to be a thing to do. */
function pressedOf(item: WaitingItem, job: string): NeedsYouAct {
  switch (item.source) {
    case "plan":
      return {
        kind: "open_plan",
      };
    case "sketch":
      return {
        kind: "open_sketch",
        title: item.id.replace(/^sketch:/, ""),
      };
    case "document":
      return {
        kind: "open_document",
        path: item.id.replace(/^document:/, ""),
      };
    case "draft":
      return {
        kind: "open_draft",
        row: item.id.replace(/^draft:/, ""),
      };
  }
  if ((item.options ?? []).length > 0)
    return {
      kind: "answer",
      options: item.options ?? [],
      other: !CHOICE_ONLY.includes(item.source),
    };
  const command = item.act?.kind === "run" ? item.act.target : undefined;
  if (command?.startsWith("scripts/preview --restart") === true)
    return {
      kind: "restart",
      label: "Refresh preview",
      build: "preview",
      adopt: false,
      command,
    };
  if ((item.steps ?? []).length > 0)
    return {
      kind: "steps",
      steps: item.steps ?? [],
      ...(command === undefined
        ? {}
        : {
            command,
          }),
    };
  const words = item.source === "agent" || item.source === "replied";
  if (item.act === undefined)
    return words
      ? {
          kind: "reply",
        }
      : {
          kind: "none",
        };
  switch (item.act.kind) {
    case "walk":
      return {
        kind: "walk",
        url: item.act.target,
      };
    case "answer":
      return words
        ? {
            kind: "reply",
          }
        : {
            kind: "answer",
            options: [],
            other: false,
          };
    case "approve_pr":
      return {
        kind: "approve_pr",
        target: item.act.target,
      };
    case "run":
      return {
        kind: "run",
        command: item.act.target,
      };
    case "message":
      return item.act.target === job
        ? {
            kind: "message",
          }
        : {
            kind: "open_job",
            id: item.act.target,
          };
  }
}

/** An agent's item that asks nothing and runs nothing is something he does, then marks done. */
function actOf(item: WaitingItem, job: string): NeedsYouAct {
  const act = pressedOf(item, job);
  const answers = item.act?.kind === "answer";
  const toDo =
    item.source === "agent" &&
    (item.options ?? []).length === 0 &&
    !answers &&
    ![
      "walk",
      "run",
      "restart",
      "approve_pr",
    ].includes(act.kind) &&
    !(act.kind === "steps" && act.command !== undefined);
  return toDo
    ? {
        kind: "done",
        ...(item.steps === undefined
          ? {}
          : {
              steps: item.steps,
            }),
      }
    : act;
}

const GLYPH: Partial<Record<string, NeedsYouGlyphKind>> = {
  plan: "plan",
  sketch: "sketch",
  document: "document",
  draft: "draft",
  walk: "walk",
  window_ask: "walk",
  held_child: "linked_job",
};

function kindOf(item: WaitingItem, act: NeedsYouAct): ToReviewKind {
  switch (item.source) {
    case "plan":
    case "sketch":
    case "document":
    case "draft":
    case "ready_to_try":
    case "goal_met":
      return item.source;
    case "job_conflict":
    case "clash_order":
      return "clash";
    case "walk":
    case "window_ask":
      return "walk";
  }
  if (act.kind === "run" || act.kind === "steps") return "command";
  if (act.kind === "done") return "to_do";
  if (act.kind === "restart") return "restart";
  if (act.kind === "approve_pr") return "pull_request";
  return "question";
}

function answerOf(
  item: WaitingItem,
  act: NeedsYouAct,
  thread: SessionThread,
  mac: FixtureMac,
): ToReviewAnswer {
  switch (item.source) {
    case "plan":
    case "sketch":
    case "document":
    case "draft":
    case "walk":
      return {
        op: "answer_items",
      };
    case "ready_to_try":
      return {
        op: "answer_ready_to_try",
        cards: (mac.tries?.[thread.session.id] ?? [])
          .filter((card) => card.state === "open")
          .map((card) => ({
            branch: card.branch,
            tip: card.tip,
            about: card.about,
          })),
      };
    case "first_message":
      return {
        op: "begin_session",
        session_id: thread.session.id,
      };
    case "held_child":
      return item.act === undefined
        ? {
            op: "press",
          }
        : {
            op: "begin_session",
            session_id: item.act.target,
          };
    case "done":
      return {
        op: "dismiss_waiting",
      };
    case "agent":
      if (item.approves === true)
        return {
          op: "answer_items",
        };
      return act.kind === "done"
        ? {
            op: "done_waiting",
          }
        : act.kind === "answer" || act.kind === "reply"
          ? {
              op: "answer_waiting",
            }
          : {
              op: "press",
            };
  }
  return {
    op: "answer_waiting",
  };
}

const dismissOf = (item: WaitingItem): NeedsYouDismiss =>
  item.source === "plan" || item.source === "first_message" ? "none" : "plain";

/** The list `GET /api/sessions/to_review` answers for this fixture Mac. */
export function toReviewOf(mac: FixtureMac): ToReview {
  const groups: ToReviewGroup[] = [];
  const into = (head: ToReviewHead, item: ToReviewItem) => {
    const key = JSON.stringify(head);
    const kept = groups.find((one) => JSON.stringify(one.head) === key);
    if (kept === undefined)
      groups.push({
        head,
        items: [
          item,
        ],
      });
    else kept.items.push(item);
  };
  for (const session of mac.live) {
    const thread = mac.threads[session.id];
    if (thread === undefined || thread.session.state === "ended") continue;
    const filed = FILED[session.id];
    (thread.session.waiting_for ?? []).forEach((one, at) => {
      const act = actOf(one, session.id);
      const item: ToReviewItem = {
        kind: kindOf(one, act),
        row: {
          id: one.id,
          source: one.source,
          text: one.text,
          since: one.since,
          glyph: {
            kind:
              GLYPH[one.source] ??
              (act.kind === "answer" ? "question" : act.kind === "run" ? "run" : "reply"),
            said: one.text,
          },
          act,
          dismiss: dismissOf(one),
        },
        order: at + 1,
        ...(one.why === undefined
          ? {}
          : {
              why: one.why,
            }),
        job: {
          id: session.id,
          title: session.title ?? session.id,
        },
        project: "armada",
        ...(filed === "sub_project"
          ? {
              sub_project: SUB_PROJECT,
            }
          : {}),
        ...(filed === "feature"
          ? {
              features: [
                POCKET,
              ],
            }
          : {}),
        answer: answerOf(one, act, thread, mac),
        ...(one.operator_read === undefined
          ? {}
          : {
              operator_read: one.operator_read,
            }),
      };
      into(
        filed === "sub_project"
          ? {
              kind: "sub_project",
              ...SUB_PROJECT,
            }
          : filed === "feature"
            ? {
                kind: "feature",
                ...POCKET,
              }
            : {
                kind: "neither",
              },
        item,
      );
    });
  }
  const rank = (head: ToReviewHead) =>
    head.kind === "sub_project" ? 0 : head.kind === "feature" ? 1 : 2;
  return {
    groups: groups.sort((a, b) => rank(a.head) - rank(b.head)),
  };
}
