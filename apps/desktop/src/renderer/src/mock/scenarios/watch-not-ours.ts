// A Job whose QA deploy watch ended not ours: `build` passed, `provision` failed on a cloud lookup the change never touched, and the agent
// set it aside. The watch is never green: it ended not ours, the reading keeps its failed part, and Fleet put it in Needs you with Ignore,
// Fix it and Escalate. The walk `watchNotOurs` plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";
import type { WatchReading, WatchSummary } from "@armada/protocol";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const OWNER = repository().manifest!.id;
const ROOT = repository().root;
const RUN = "https://ci.example.com/runs/5120";
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SET_ASIDE = "the cloud lookup for the QA region failed, and this change touches no infrastructure";
const ENDED = ago(6);

const readings: WatchReading[] = [
  { kind: "read", at: ago(24), state: "running", finished: false, says: "Build running", link: RUN, parts: [{ name: "build", state: "running" }, { name: "provision", state: "waiting" }] },
  {
    kind: "read",
    at: ENDED,
    state: "failed",
    finished: true,
    says: "provision failed: cloud lookup for region qa-east returned no account",
    link: RUN,
    parts: [
      { name: "build", state: "passed" },
      { name: "provision", state: "failed" },
    ],
  },
];

const WATCH_ID = "w-qa-filters";
const SESSION_ID = "wn-qa";

const deploy: WatchSummary = {
  id: WATCH_ID,
  session_id: SESSION_ID,
  name: "QA deploy",
  command: ["scripts/watch/deploy-state", "7c1e0b4", "qa"],
  recipe: "qa-deploy",
  every_seconds: 60,
  gives_up_after_seconds: 7200,
  follows: 431,
  state: "not_ours",
  live: false,
  started_at: ago(26),
  last: readings[1]!,
  readings,
  set_aside: [{ part: "provision", sentence: SET_ASIDE }],
  verdicts: [{ at: ago(5), verdict: "not_ours", sentence: SET_ASIDE, parts: ["provision"] }],
};

const job: Session = {
  id: SESSION_ID,
  title: "Ship the search filters to QA",
  manifestId: OWNER,
  cwd: ROOT,
  turn: { state: "idle" },
  standing: "idle",
  lastTurnAt: ago(5),
  rows: [
    { id: "wn-1", at: "14:20:00", kind: "message", from: { kind: "you" }, text: "Ship the search filters and see them through the QA deploy." },
    { id: "wn-2", at: "14:38:40", kind: "message", from: { kind: "agent" }, text: "The QA deploy ended not ours: provision failed on a cloud lookup this change does not touch. It did not pass." },
  ],
  attachments: [
    { kind: "branch", name: "web/search-filters", slot: 4 },
    { kind: "pull_request", number: 431, title: "Filter search results by owner and date", branch: "web/search-filters", address: "https://git.example/armada/pull/431", checks: { state: "passed" }, state: "open", auto: false },
  ],
  work: { proposal: { workflow: null } },
  watches: [deploy],
  waitingFor: [
    {
      id: `watch_not_ours:${WATCH_ID}`,
      text: "QA deploy: provision failed",
      since: ENDED,
      source: "watch_not_ours",
      act: { kind: "answer", target: WATCH_ID },
      options: [{ label: "Ignore" }, { label: "Fix it" }],
    },
  ],
};

const base = holding("watch-not-ours", "A Job whose QA deploy watch ended not ours, with the set-aside failure in Needs you");

export const watchNotOurs: Scenario = {
  ...base,
  state: { ...base.state, repository: null },
  draft: { sessions: () => sessionsStore([job], [], new Map(), []) },
};
