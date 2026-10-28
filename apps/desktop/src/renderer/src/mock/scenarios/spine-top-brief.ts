// The top of the Job's story: a header band with the workflow's name, its picker and its steps (two takes of the steps, `?take=labelled` and `?take=glyphs`), and the Brief as
// the first beat on the spine above Completed, which opens in place to the whole text and how the Job started. Over `job-story-full`, whose Job gets a six-step workflow at
// its fourth step, a long brief and the settings it started with, one of them the Job it was started from. Mock only; the `spineTopBrief` walk and
// `?scenario=spine-top-brief&take=` open it.

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

const brief: SessionRow = { id: "brief-1", at: "09:02:00", kind: "message", from: { kind: "you" }, text: BRIEF };

export const s335SpineTopBrief: Scenario = {
  ...base,
  name: "spine-top-brief",
  says: "A Job at the Walk step of a six-step workflow, with a long brief and the settings it started with, one of them the Job it was started from. ?take=labelled or ?take=glyphs draws the steps",
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
          work: { ...one.work, proposal: { workflow: FLOW.id, picked: ["workflow"] }, workflow: { ...FLOW, at: 3 } },
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
