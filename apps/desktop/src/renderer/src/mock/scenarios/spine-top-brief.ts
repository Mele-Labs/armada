// The top of the Job's story: a header band with the workflow's name and its picker, the Brief as
// the first beat on the spine, which opens in place to the whole text and how the Job started, then one zone for each of the workflow's steps. Over `job-story-full`, whose Job gets a six-step workflow at
// its fourth step, a long brief and the settings it started with, one of them the Job it was started from. Mock only; the `spineTopBrief` walk and
// `?scenario=spine-top-brief` open it.

import type { Session, SessionRow, WorkflowChoice } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s334JobStoryFull as base } from "./job-story-full";

const BUSY = "one-view-3";

const FLOW: WorkflowChoice = { id: "Product", name: "Product", steps: ["Propose", "Discuss", "Approve plan", "Walk", "Refine", "Deliver"] };
const OTHERS: readonly WorkflowChoice[] = [
  { id: "Feature", name: "Feature", steps: ["Plan", "Build", "Verify", "Pull request"] },
  { id: "Bug fix", name: "Bug fix", steps: ["Reproduce", "Fix", "Verify", "Pull request"] },
];

const BRIEF = [
  "Add an Issues tab beside Reviews. It lists the open issues of the repository the Job is in, searchable in the tracker's own query language, and each row opens the issue where the tracker shows it.",
  "A saved query should survive a restart and belong to the repository it was written for. A query that matches nothing says so in the list and never reads as an error.",
  "Start a Job from a ticket with one press, carrying the ticket's words as the brief. Keep the first version to Jira and GitHub, and leave the settings page for a later Job.",
].join("\n\n");

// When each step began. The Job went back from Approve plan to Discuss once, so Discuss and Approve plan are each visited twice.
const VISITS = [
  { step: 0, since: "2026-10-27T09:02:00Z" },
  { step: 1, since: "2026-10-27T09:10:00Z" },
  { step: 2, since: "2026-10-27T09:40:00Z" },
  { step: 1, since: "2026-10-27T09:55:00Z" },
  { step: 2, since: "2026-10-27T10:20:00Z" },
  { step: 3, since: "2026-10-27T10:40:00Z" },
] as const;

/** Where each group the Job finished stood: the tasks' last change, and the pull request's time by number. */
const DONE_AT: Record<string, string> = { g2: "2026-10-27T09:30:00Z", g6: "2026-10-27T10:05:00Z", g3: "2026-10-27T10:30:00Z" };

const brief: SessionRow = { id: "brief-1", at: "09:02:00", kind: "message", from: { kind: "you" }, text: BRIEF };

export const s335SpineTopBrief: Scenario = {
  ...base,
  name: "spine-top-brief",
  says: "A Job at the Walk step of a six-step workflow, whose steps are the zones of its story, with a long brief and the settings it started with, one of them the Job it was started from.",
  draft: {
    ...base.draft,
    sessions: (control) => {
      const store = base.draft!.sessions!(control);
      store.edit(
        BUSY,
        (one): Session => ({
          ...one,
          rows: [brief, ...one.rows],
          model: "opus",
          effort: "high",
          mode: "auto",
          started: { workflow: "Product", delivery: "open_pr", model: "opus", effort: "high", mode: "auto", by: { job: "one-view-1", title: "Retry the slot lease" } },
          trail: { started: "2026-10-27T09:02:00Z", ...one.trail, opened: { 3011: "2026-10-27T09:35:00Z", 3012: "2026-10-27T10:10:00Z", 3013: "2026-10-27T10:32:00Z" } },
          work: {
            ...one.work,
            proposal: { workflow: FLOW.id, picked: ["workflow"] },
            workflow: { ...FLOW, at: 3, visits: VISITS },
            ...(one.work?.plan === undefined ? {} : { plan: { ...one.work.plan, groups: one.work.plan.groups.map((group) => (DONE_AT[group.id] === undefined ? group : { ...group, tasks: group.tasks.map((task) => ({ ...task, changedAt: DONE_AT[group.id]! })) })) } }),
          },
        }),
      );
      const choices = [FLOW, ...OTHERS];
      return {
        ...store,
        work: {
          ...store.work!,
          workflows: choices,
          choose: (id, name) => {
            const chosen = choices.find((one) => one.id === name);
            store.edit(id, (one): Session => {
              const { workflow, ...rest } = one.work ?? {};
              void workflow;
              return { ...one, work: { ...rest, proposal: { workflow: name }, ...(chosen === undefined ? {} : { workflow: { ...chosen, at: Math.min(one.work?.workflow?.at ?? 0, chosen.steps.length - 1) } }) } };
            });
          },
        },
      };
    },
  },
};
