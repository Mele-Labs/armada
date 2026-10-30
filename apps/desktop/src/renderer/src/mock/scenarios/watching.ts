// Two Jobs with watches. One in the middle of a QA deploy, its agent at work: a QA deploy watch that failed, where the agent set one failing
// part aside as not its change's doing and called the other its own; a QA health watch still running; and the image build's watch, passed.
// The other's turn has ended on a merged pull request with a Staging deploy still running, so that watch is all that holds it open.
// Stop on it ends the watch and lets the Job go, as Fleet does. The walk `watching` plays it.
// QA health has read the same nine times in a row, which Fleet sends as one run; the walk `watchingTidy` plays that and the ended watches.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";
import type { WatchReading, WatchSummary } from "@armada/protocol";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const CI = "https://ci.example.com/runs/";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const pr = (number: number, title: string, branch: string, state: "open" | "merged"): SessionAttachment => ({
  kind: "pull_request",
  number,
  title,
  branch,
  address: `https://git.example/armada/pull/${number}`,
  checks: { state: "passed" },
  state,
  auto: false,
});

/** The newest reading as taken, as Fleet sends `last`: a run's last reading, never the run. */
const taken = (reading: WatchReading): WatchReading => {
  const { count, last_at, ...one } = reading;
  void count;
  return last_at === undefined ? one : { ...one, at: last_at };
};

const watch = (id: string, sessionId: string, name: string, rest: Omit<WatchSummary, "id" | "session_id" | "name" | "last">): WatchSummary => {
  const last = rest.readings?.at(-1);
  return { id, session_id: sessionId, name, ...rest, ...(last === undefined ? {} : { last: taken(last) }) };
};

const qaDeployReadings: WatchReading[] = [
  { kind: "read", at: ago(50), state: "running", finished: false, says: "Build running", link: `${CI}4409`, parts: [{ name: "build", state: "running" }] },
  {
    kind: "read",
    at: ago(31),
    state: "running",
    finished: false,
    says: "Deploy running, one of two services done",
    link: `${CI}4409`,
    parts: [
      { name: "build", state: "passed" },
      { name: "deploy-web", state: "passed" },
      { name: "deploy-api", state: "running" },
      { name: "unit-tests-billing", state: "running" },
    ],
  },
  {
    kind: "read",
    at: ago(12),
    state: "failed",
    finished: true,
    says: "deploy-api and unit-tests-billing failed",
    link: `${CI}4409`,
    parts: [
      { name: "build", state: "passed" },
      { name: "deploy-web", state: "passed" },
      { name: "deploy-api", state: "failed" },
      { name: "unit-tests-billing", state: "failed" },
    ],
  },
];

const BILLING = "a unit test in a service this change does not touch";

const QA_ID = "wa-qa";
const qa: Session = {
  id: QA_ID,
  title: "Ship the rate limit to QA",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "working" },
  standing: "working",
  lastTurnAt: ago(14),
  rows: [
    { id: "wa-qa-1", at: "13:02:00", kind: "message", from: { kind: "you" }, text: "Ship the rate limit and see it through the QA deploy." },
    { id: "wa-qa-2", at: "13:58:10", kind: "message", from: { kind: "agent" }, text: "The QA deploy failed on deploy-api. The API's QA values lack the new setting; adding it." },
  ],
  attachments: [{ kind: "branch", name: "api/rate-limit", slot: 2 }, pr(412, "Limit requests per key on the public API", "api/rate-limit", "open")],
  work: { proposal: { workflow: null } },
  watches: [
    watch("w-health", QA_ID, "QA health", {
      command: ["scripts/watch/health", "qa", "/api/limits"],
      recipe: "qa-health",
      every_seconds: 60,
      gives_up_after_seconds: 7200,
      state: "running",
      live: true,
      started_at: ago(11),
      readings: [
        { kind: "read", at: ago(10), state: "waiting", finished: false, says: "Waiting for the deploy to finish" },
        { kind: "read", at: ago(9), state: "running", finished: false, says: "Two of three regions answering", count: 9, last_at: ago(1) },
      ],
    }),
    watch("w-deploy", QA_ID, "QA deploy", {
      command: ["scripts/watch/deploy-state", "4f2c9e1", "qa"],
      recipe: "qa-deploy",
      every_seconds: 60,
      gives_up_after_seconds: 7200,
      follows: 412,
      state: "failed",
      live: false,
      started_at: ago(52),
      readings: qaDeployReadings,
      set_aside: [{ part: "unit-tests-billing", sentence: BILLING }],
      verdicts: [
        { at: ago(11), verdict: "not_ours", sentence: BILLING, parts: ["unit-tests-billing"] },
        { at: ago(10), verdict: "ours", sentence: "the API's QA values lack the new rate limit setting" },
      ],
    }),
    watch("w-image", QA_ID, "Build image", {
      command: ["scripts/watch/image-state", "4f2c9e1"],
      every_seconds: 30,
      gives_up_after_seconds: 1800,
      state: "passed",
      live: false,
      started_at: ago(58),
      readings: [
        { kind: "read", at: ago(57), state: "running", finished: false, says: "Building the image" },
        { kind: "read", at: ago(53), state: "passed", finished: true, says: "Image built and pushed" },
      ],
    }),
  ],
};

const HELD_ID = "wa-held";
const held: Session = {
  id: HELD_ID,
  title: "Raise the upload limit",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  standing: "idle",
  lastTurnAt: ago(18),
  rows: [
    { id: "wa-h-1", at: "12:40:00", kind: "message", from: { kind: "you" }, text: "Raise the upload limit to 50 MB and see it deployed to staging." },
    { id: "wa-h-2", at: "13:44:30", kind: "message", from: { kind: "agent" }, text: "#408 merged. The Staging deploy is running; I will carry on when it settles." },
  ],
  attachments: [{ kind: "branch", name: "api/upload-limit", slot: 3 }, pr(408, "Raise the upload limit to 50 MB", "api/upload-limit", "merged")],
  work: { proposal: { workflow: null } },
  heldOpenBy: [{ kind: "watch", name: "Staging deploy" }],
  watches: [
    watch("w-staging", HELD_ID, "Staging deploy", {
      command: ["scripts/watch/deploy-state", "9b71d03", "staging"],
      recipe: "staging-deploy",
      every_seconds: 60,
      gives_up_after_seconds: 7200,
      follows: 408,
      state: "running",
      live: true,
      started_at: ago(17),
      readings: [
        { kind: "read", at: ago(16), state: "waiting", finished: false, says: "Queued behind another deploy", link: `${CI}4417` },
        { kind: "read", at: ago(2), state: "running", finished: false, says: "Deploy running, one of three services done", link: `${CI}4417`, parts: [{ name: "deploy-api", state: "passed" }, { name: "deploy-web", state: "running" }, { name: "deploy-worker", state: "waiting" }] },
      ],
    }),
  ],
};

const base = holding("watching", "A Job in the middle of a QA deploy with a watch running, one failed and weighed by its agent, and one passed; and a Job a Staging deploy watch holds open");

export const watching: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: { sessions: () => sessionsStore([qa, held], [], new Map(), []) },
};
