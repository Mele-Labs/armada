// A Job whose members are Jobs, read for the board that draws them.
//
// Everything here is arithmetic and copy over `draft/members.ts` and
// `draft/landing.ts`: which branch each member targets, what joins one to the
// next, what finishes the parent, and what answering one member's question
// moves. It is a module of its own so those can be tested a hundred cases at
// a time — a `play` that computed them would be a unit test paying a browser's
// price.

import { useState } from "react";

import type { JudgeAnswer } from "@armada/protocol";
import { JOB_STATUS } from "@armada/components";
import type {
  JobMemberJoin,
  JobMemberRow,
  JobMembersProps,
  JobMemberState,
  MemberDecisionProps,
  TaskBarSegment,
} from "@armada/components";

import type { JobMembersView, MemberView } from "./draft/members";
import type { LandingRule } from "./draft/landing";

/** What the Overview region needs from the screen to draw an act. */
export type MemberActs = {
  /** Open one member's own Job. */
  onOpenJob?: (jobId: string) => void;
  /**
   * Open one member's pull request where it lives. **Takes the member's Job
   * id, never the address the card drew** — `opening.ts`'s rule, so the string
   * on screen cannot decide what opens.
   */
  onOpenPullRequest?: (jobId: string) => void;
  /**
   * Answer the Judge refusal a member is holding — `answer_judge`, against
   * that member's Job id. The same verdict as answering on its own screen.
   */
  onAnswerJudge?: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string) => void;
  /**
   * Drop a member. **Offered only where the caller hands one in**, because no
   * operation serves it: Fleet neither closes a pull request nor rebases what
   * was stacked on the branch.
   */
  onDropMember?: (jobId: string, reason: string) => void;
  /** Whether this Job is live, and whether something is already on its way. */
  stale?: boolean;
  acting?: boolean;
};

/**
 * The whole board: the order, the one decision open on it, and the line that
 * leads the screen.
 *
 * **The banner is what leads**, because the thing waiting on a person is what
 * they opened the Job to find — a screen that opens on three rows of facts
 * makes them look for it.
 */
export type MembersBoard = {
  train: JobMembersProps;
  /** The member holding a question, where one is. */
  decision?: MemberDecisionProps;
  /** What leads the screen: the one thing outstanding, or that nothing is. */
  lead: { said: string; because: string; waiting: boolean };
};

/**
 * The board's whole input, or nothing where this Job has no members.
 *
 * **Nothing draws for an ordinary Job**, which is every Job Fleet runs today:
 * one Job, one pull request, and no order to read.
 */
export function membersOf(
  view: JobMembersView | undefined,
  landing: LandingRule | undefined,
  acts: MemberActs = {},
): MembersBoard | undefined {
  if (view === undefined || view.members.length === 0) return undefined;
  const counted = view.members.filter((member) => member.dropped === undefined);
  const asking = view.members.findIndex(
    (member) => member.question !== undefined && member.dropped === undefined,
  );
  return {
    train: {
      members: view.members.map((member, at) => rowOf(member, at, view.members, landing, acts)),
      joins: view.members.slice(1).map((member, at) => joinOf(member, at + 1, landing)),
      said: saidOf(counted.length),
      waiting: view.members.filter(
        (member) => member.question !== undefined && member.dropped === undefined,
      ).length,
      completeWhen: completeWhenSaid(view.members, landing),
      completeBar: counted.map((member): TaskBarSegment => (member.landed ? "done" : "open")),
    },
    ...(asking === -1
      ? {}
      : { decision: decisionOf(view.members[asking] as MemberView, asking, view.members, acts) }),
    lead: leadOf(view.members, asking, landing),
  };
}

/** What the set is, beside its name. Every member has both, by construction. */
function saidOf(count: number): string {
  return `${count} members, a worktree and a pull request each`;
}

function rowOf(
  member: MemberView,
  at: number,
  all: readonly MemberView[],
  landing: LandingRule | undefined,
  acts: MemberActs,
): JobMemberRow {
  const targets = targetOf(member, all[at - 1], landing);
  const label = pullRequestLabel(member.pull_request);
  const said = tasksSaid(member);
  const canDrop =
    acts.onDropMember !== undefined && !member.landed && member.dropped === undefined;
  return {
    id: member.job,
    ordinal: at + 1,
    title: member.title,
    state: stateOf(member.status),
    landed: member.landed,
    ...(member.link === undefined ? {} : { link: member.link }),
    // Only a stacked member's target is worth a reader's eye: everything else
    // targets where the Job lands, which the foot already says once.
    ...(member.link === "stacked" && targets !== undefined ? { targets } : {}),
    ...(member.pull_request === undefined ? {} : { pullRequest: member.pull_request }),
    ...(label === undefined ? {} : { pullRequestLabel: label }),
    ...(member.pull_request === undefined
      ? {}
      : { pullRequestState: pullRequestState(member, landing) }),
    ...(member.branch === undefined ? {} : { branch: member.branch }),
    ...(member.scope === undefined ? {} : { scope: member.scope }),
    ...(tasksOf(member) === undefined ? {} : { tasks: tasksOf(member) }),
    ...(said === undefined ? {} : { tasksSaid: said }),
    ...(member.dropped === undefined ? {} : { dropped: member.dropped }),
    ...(member.question === undefined || member.dropped !== undefined
      ? {}
      : { asked: askedSaid(member) }),
    ...(acts.onOpenJob === undefined ? {} : { onOpen: () => acts.onOpenJob?.(member.job) }),
    ...(acts.onOpenPullRequest === undefined
      ? {}
      : { onOpenPullRequest: () => acts.onOpenPullRequest?.(member.job) }),
    ...(canDrop ? { onDrop: (reason: string) => acts.onDropMember?.(member.job, reason) } : {}),
  };
}

/** The decision column: one member's refusal, read and answered beside the order. */
function decisionOf(
  member: MemberView,
  at: number,
  all: readonly MemberView[],
  acts: MemberActs,
): MemberDecisionProps {
  const question = member.question as NonNullable<MemberView["question"]>;
  const base = JOB_STATUS[member.status];
  return {
    who: `Member ${at + 1}`,
    criterion: question.criterion_id,
    question: question.question,
    expected: question.expected,
    produced: question.produced,
    consequence: question.consequence,
    ...(base?.verb == null ? {} : { state: base.verb }),
    moves: movesSaid(all.slice(at + 1)),
    disabled: acts.stale === true || acts.acting === true,
    ...(acts.stale === true
      ? { disabledNote: "This job is not live, so nothing can be sent." }
      : acts.acting === true
        ? { disabledNote: "Something sent to this job is still on its way to Fleet." }
        : {}),
    onAnswer: (answer: JudgeAnswer, note?: string) =>
      acts.onAnswerJudge?.(member.job, question.asked_at, answer, note),
    ...(acts.onOpenJob === undefined ? {} : { onOpen: () => acts.onOpenJob?.(member.job) }),
  };
}

/**
 * What leads the screen.
 *
 * **A screen never says a thing it does not know.** Where nothing is asking,
 * the lead says so rather than inventing urgency; where one member is, it
 * names that member and what the press moves.
 */
function leadOf(
  all: readonly MemberView[],
  asking: number,
  landing: LandingRule | undefined,
): MembersBoard["lead"] {
  const counted = all.filter((member) => member.dropped === undefined);
  const landed = counted.filter((member) => member.landed).length;
  const count = `${landed} of ${counted.length} pull ${counted.length === 1 ? "request" : "requests"} merged.`;
  if (asking === -1) {
    return { said: "Nothing is waiting on you.", because: count, waiting: false };
  }
  const member = all[asking] as MemberView;
  const behind = movesSaid(all.slice(asking + 1));
  return {
    said: `Member ${asking + 1} is asking you something.`,
    because: `${openSaid(member, landing)} ${behind} ${count}`,
    waiting: true,
  };
}

/** Where a member's own pull request stands, in one clause. */
function openSaid(member: MemberView, landing: LandingRule | undefined): string {
  if (member.pull_request === undefined) return "It has opened no pull request yet.";
  return `Its pull request is ${pullRequestState(member, landing)}.`;
}

/**
 * What the pull request is.
 *
 * **Three words and no fourth.** Nothing on the wire says whether a pull
 * request's checks passed, so the word is what the Job's own record answers:
 * merged where `JobDelivery.landed` says so, draft where the Job was dispatched
 * to park it, and open otherwise.
 */
function pullRequestState(member: MemberView, landing: LandingRule | undefined): string {
  if (member.landed) return "merged";
  return landing?.pr_mode === "draft" ? "draft" : "open";
}

/**
 * What a member is holding a person up on, on its own row.
 *
 * The criterion in the wire's spelling, then the question. It says a question
 * is open and nothing about how to answer it, which is the column beside it.
 */
function askedSaid(member: MemberView): string {
  const question = member.question as NonNullable<MemberView["question"]>;
  return `The Judge refused ${question.criterion_id}. ${question.question}`;
}

/**
 * The line between one member and the one before it.
 *
 * **It names both, because the edge belongs to neither.** `Parked until the
 * one before it lands` sat at the top of the later card and read as a property
 * of that card; what a reader is being told is how two of them relate.
 */
function joinOf(
  member: MemberView,
  at: number,
  landing: LandingRule | undefined,
): JobMemberJoin | undefined {
  const link = member.link;
  if (link === undefined) return undefined;
  const me = `Member ${at + 1}`;
  const before = `member ${at}`;
  const lands = landing?.target;
  if (link === "stacked") {
    return { link, said: `${me} branches off ${before}, so it keeps working.` };
  }
  if (link === "published") {
    return { link, said: `${me} waits on what ${before}'s merge publishes.` };
  }
  return {
    link,
    said:
      lands === undefined
        ? `${me} waits for ${before} to merge.`
        : `${me} targets ${lands}, so it waits for ${before} to merge.`,
  };
}

/**
 * How a status reads.
 *
 * **The verb, the glyph and the token are the registry's** — a status this
 * build has no row for renders its wire spelling and says so, rather than
 * borrowing a word from a status that happens to look near it.
 */
function stateOf(status: string): JobMemberState {
  const base = JOB_STATUS[status];
  if (base === undefined) {
    return { as: "text", wire: status, missing: `No row in the registry for ${status}` };
  }
  if (base.verb === null || base.icon === null || base.badgeStatus === null) {
    return { as: "text", wire: status, missing: `No verb or glyph in the registry for ${status}` };
  }
  return { as: "badge", status: base.badgeStatus, icon: base.icon, label: base.verb };
}

/**
 * Which branch a member's pull request targets.
 *
 * **A stacked member targets the branch before it; everything else targets
 * where the Job lands** — the issue's own rule, and `published` is not
 * stacked, so it lands where the Job does and waits on a release besides.
 */
function targetOf(
  member: MemberView,
  before: MemberView | undefined,
  landing: LandingRule | undefined,
): string | undefined {
  if (member.link === "stacked") return before?.branch;
  return landing?.target ?? undefined;
}

/**
 * A pull request by its number rather than its whole address — the last
 * segment of the path. A URL drawn in full is a line of chrome across a row.
 */
function pullRequestLabel(address: string | undefined): string | undefined {
  if (address === undefined) return undefined;
  const last = address.split("/").filter((part) => part !== "").pop();
  return last === undefined ? undefined : `#${last}`;
}

/**
 * A member's plan as one segment per task. **Dropped tasks are out**, the
 * count the plan region already draws: a dropped task is not work owed.
 */
function tasksOf(member: MemberView): readonly TaskBarSegment[] | undefined {
  const counts = member.tasks;
  if (counts === undefined) return undefined;
  const bar: TaskBarSegment[] = [
    ...Array.from({ length: counts.done }, (): TaskBarSegment => "done"),
    ...Array.from({ length: counts.working }, (): TaskBarSegment => "working"),
    ...Array.from({ length: counts.open }, (): TaskBarSegment => "open"),
  ];
  return bar.length === 0 ? undefined : bar;
}

/** The same plan counted, ending the row. */
function tasksSaid(member: MemberView): string | undefined {
  const bar = tasksOf(member);
  if (bar === undefined) return undefined;
  return `${bar.length} ${bar.length === 1 ? "task" : "tasks"}`;
}

/**
 * What finishes the parent, and how far along it is.
 *
 * **`all_members_landed` is the default where the Job said nothing**, because
 * a Job with members is done when its members are (#1530). Landed is the
 * merge, so the count is of pull requests in and never of Jobs that reached a
 * successful status.
 */
export function completeWhenSaid(
  members: readonly MemberView[],
  landing: LandingRule | undefined,
): string {
  const counted = members.filter((member) => member.dropped === undefined);
  const landed = counted.filter((member) => member.landed).length;
  const count = `${landed} of ${counted.length} pull ${counted.length === 1 ? "request" : "requests"} merged.`;
  const rule = landing?.complete_when;
  if (rule === "pr_merged") return `This job's own pull request merges. ${count}`;
  if (rule === "pr_opened") return `This job's pull request is open. ${count}`;
  if (rule === "delivered") return `The delivering step has delivered. ${count}`;
  return `Every member has landed. ${count}`;
}

/**
 * What answering one member's question moves. **Named, not counted alone** —
 * the press stops or advances work somebody else is waiting on, and which
 * work that is is the thing a person needs before pressing.
 */
export function movesSaid(behind: readonly MemberView[]): string {
  const waiting = behind.filter((member) => member.dropped === undefined);
  if (waiting.length === 0) return "Nothing else is waiting on this answer.";
  const titles = waiting.map((member) => member.title).join(", ");
  const said =
    waiting.length === 1
      ? "1 pull request behind this one is waiting on the answer"
      : `${waiting.length} pull requests behind this one are waiting on the answer`;
  return `${said}: ${titles}.`;
}

/**
 * Which members somebody dropped in this window, and the act that drops one.
 *
 * **Held here because no operation serves it.** Fleet neither closes a pull
 * request nor rebases what was stacked on the branch, so a drop is what this
 * window draws and nothing more — which is why only a reading the mock handed
 * in offers the control at all. It goes the day `drop_member` exists.
 */
export function useDroppedMembers(): {
  over: (view: JobMembersView | undefined) => JobMembersView | undefined;
  drop: (jobId: string, reason: string) => void;
} {
  const [dropped, setDropped] = useState<Readonly<Record<string, string>>>({});
  return {
    over: (view) =>
      view === undefined
        ? undefined
        : {
            ...view,
            members: view.members.map((member) => {
              const reason = dropped[member.job];
              return reason === undefined ? member : { ...member, dropped: reason };
            }),
          },
    drop: (jobId, reason) => setDropped((held) => ({ ...held, [jobId]: reason })),
  };
}
