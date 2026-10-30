// A Job that orchestrates: its drafted plan has four groups, each for a Job of its own, and it holds the five issues it was given. Before they are handed, each group is the
// Job card alone as a placeholder, its issue a chip in it, in the rail and on the thread's plan card. Time passing approves the plan, moves the Job to Dispatch and hands the
// first two groups to Jobs, whose real Job cards take their places; the other two stay placeholders. Open holds only the issue no group names throughout. Mock only: `ownJob` is set here alone, and the two Jobs are in the list before the hand-off, which the store cannot add one to. The walk
// `plan-groups-as-jobs` plays it.

import type { Session, SessionAttachment, WorkGroup } from "@armada/screens/src/draft/sessions";

import { mockAreas } from "../cockpit-areas";
import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { planCall } from "../plan-call";
import { ARMADA, agent, ago, child, started, task, tool, you } from "../sessions/linked";
import { sessionsStore } from "../sessions/script";

const PARENT = "orchestrate-1";
const START_JOB = "orchestrate-2";
const RULE_NAME = "orchestrate-3";

const STEPS = ["Assess", "Plan", "Dispatch", "Oversee", "Summarise"] as const;

const issue = (number: string, title: string): SessionAttachment => ({ kind: "issue", number, title, address: `https://forge.invalid/issues/${number}`, state: "open" });

const ISSUES = [
  issue("armada-95v", "A start_job rule act ignores the workflow it names"),
  issue("armada-tzg", "A rule's row in Changed on this Job shows its id rather than its name"),
  issue("armada-7gs", "A rule proposed for every Job shows nowhere on the Workflows page"),
  issue("armada-cwj", "You can't create a Workspace or set autonomy defaults from the Workflows page"),
  // No group names this one, so Open holds it throughout.
  issue("armada-m8r", "Keep on a changed rule row stays lit after the Job ends"),
];

const groups: WorkGroup[] = [
  {
    id: "g1",
    agent: "rust-engineer",
    title: "Start the Job on the workflow a rule names",
    paths: ["crates/fleet/src/rules"],
    issue: "armada-95v",
    landsIn: { label: "PR 95v" },
    ownJob: true,
    tasks: [task("T1", "Read the workflow a start_job act names", "open"), task("T2", "Start the Job on it, not on the repository's usual one", "open")],
  },
  {
    id: "g2",
    agent: "bridge-engineer",
    title: "Name the rule in Changed on this Job",
    paths: ["apps/desktop/src/renderer/src/session-view"],
    issue: "armada-tzg",
    landsIn: { label: "PR tzg" },
    ownJob: true,
    tasks: [task("T3", "Look the rule up by its id", "open"), task("T4", "Draw its name in the row", "open")],
  },
  {
    id: "g3",
    agent: "bridge-engineer",
    title: "List a rule proposed for every Job on the Workflows page",
    paths: ["apps/desktop/src/renderer/src/workflows"],
    issue: "armada-7gs",
    landsIn: { label: "PR 7gs" },
    ownJob: true,
    tasks: [task("T5", "Read the rules proposed for every Job", "open"), task("T6", "List them under the workflow's rules", "open")],
  },
  {
    id: "g4",
    agent: "bridge-engineer",
    title: "Create a Workspace and set autonomy defaults from the Workflows page",
    paths: ["apps/desktop/src/renderer/src/workflows"],
    issue: "armada-cwj",
    landsIn: { label: "PR cwj" },
    ownJob: true,
    tasks: [task("T7", "Add a Workspace from the page", "open"), task("T8", "Set its autonomy defaults", "open")],
  },
];

const parent = (): Session => ({
  id: PARENT,
  title: "Fix four filed issues",
  manifestId: "armada",
  cwd: ARMADA.root,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  standing: "idle",
  started: { workflow: "orchestrate", delivery: "open_pr" },
  rows: [
    you("o-1", "10:02:00", "Fix these four issues, dispatching a Job for each and seeing them through"),
    agent("o-2", "10:09:40", "Four groups, one for each issue. Each goes to a Job of its own."),
    tool("o-3", "10:09:50", "record_plan"),
  ],
  attachments: ISSUES,
  waitingFor: [planCall(undefined, ago(3))],
  work: { workflow: { name: "orchestrate", steps: STEPS, at: 1 }, plan: { scope: ["crates/fleet/src/rules", "apps/desktop/src/renderer/src"], approved: false, after: "o-2", groups } },
});

/** The Jobs the first two groups go to. */
const START_JOB_TITLE = "A start_job rule act reads the workflow it names";
const RULE_NAME_TITLE = "A rule's row in Changed shows its name";

const jobs = (): Session[] => [
  child(START_JOB, START_JOB_TITLE, "g1", 3, "armada/start-job-workflow", "Make a start_job rule act read the workflow it names. Group: Start the Job on the workflow a rule names.", [], PARENT),
  child(RULE_NAME, RULE_NAME_TITLE, "g2", 4, "armada/rule-name-in-changed", "Show a rule's name in the Changed list of a Job. Group: Name the rule in Changed on this Job.", [], PARENT),
];

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  const sessions = () => {
    const store = sessionsStore([parent(), ...jobs()], [], new Map(), []);
    let handed = false;
    return {
      ...store,
      // Time passing approves the plan, moves the Job to Dispatch and hands the first two groups to Jobs.
      later: () => {
        if (handed) return;
        handed = true;
        store.edit(
          PARENT,
          (one): Session => ({
            ...one,
            lastTurnAt: new Date().toISOString(),
            rows: [
              ...one.rows,
              you("o-4", "10:14:00", "Approved."),
              tool("o-5", "10:14:10", `start_another_job ${START_JOB_TITLE}`),
              started("o-6", "10:14:11", START_JOB, START_JOB_TITLE),
              tool("o-7", "10:14:20", `start_another_job ${RULE_NAME_TITLE}`),
              started("o-8", "10:14:21", RULE_NAME, RULE_NAME_TITLE),
            ],
            attachments: [...one.attachments, { kind: "started_to", id: START_JOB, group: "g1" }, { kind: "started_to", id: RULE_NAME, group: "g2" }],
            waitingFor: [],
            work: { ...one.work, workflow: { name: "orchestrate", steps: STEPS, at: 2 }, plan: { ...one.work!.plan!, approved: true } },
          }),
        );
      },
    };
  };
  return { ...board, name: "plan-groups-as-jobs", says: "A Job at Plan whose four groups each go to a Job of their own, drawn as placeholder Job cards until the first two are handed", draft: { sessions, areas: mockAreas } };
}

export const planGroupsAsJobs: Scenario = build();
