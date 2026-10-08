// A long Session's ledger, which draws what is open and keeps the rest behind All. Over the
// `sessions` scenario with one more Session open beside its own: six open or draft pull requests
// among eleven merged ones, two running subagents among eleven finished, and Jobs landed and not.
// The walk `ledger-filter` plays it.

import type { Session, SessionAttachment } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const idle = { state: "idle" } as const;

const pull = (number: number, state: "open" | "draft" | "merged"): SessionAttachment => ({
  kind: "pull_request",
  number,
  title: `Retire the sleeps, part ${number - 1900}`,
  branch: `fix/retire-sleeps-${number - 1900}`,
  address: `https://example.com/pull/${number}`,
  checks: { state: state === "merged" ? "passed" : state === "draft" ? "pending" : "passed" },
  state,
  auto: false,
});

const subagent = (n: number, state: "running" | "done"): SessionAttachment => ({
  kind: "subagent",
  id: `sub${n}`,
  task: `Read the CI history, shard ${n}`,
  state,
});

const job = (number: number, state: "running" | "landed" | "superseded"): SessionAttachment => ({
  kind: "job",
  id: `job${number}`,
  number,
  title: `Pin the store clock, ${number}`,
  state,
  branch: `fix/pin-${number}`,
});

const more: Session[] = [
  {
    id: "s11",
    terminal: true,
    title: "Retire the sleeps",
    turn: idle,
    lastTurn: "14:40",
    rows: [],
    attachments: [
      { kind: "branch", name: "fix/retire-sleeps", slot: 2 },
      ...[1901, 1902, 1903, 1904, 1905, 1906, 1907, 1908, 1909, 1910, 1911].map((n) => pull(n, "merged")),
      pull(1912, "open"),
      pull(1913, "open"),
      pull(1914, "draft"),
      pull(1915, "open"),
      pull(1916, "draft"),
      pull(1917, "open"),
      job(61, "landed"),
      job(62, "superseded"),
      job(63, "running"),
      ...[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11].map((n) => subagent(n, "done")),
      subagent(12, "running"),
      subagent(13, "running"),
      { kind: "artifact", form: "doc", id: "https://example.com/artifact/sleeps", title: "Sleeps write-up" },
    ],
  },
  {
    id: "s12",
    terminal: true,
    title: "Settled work",
    turn: idle,
    lastTurn: "14:12",
    rows: [],
    attachments: [pull(1920, "merged"), pull(1921, "merged"), subagent(1, "done")],
  },
];

export const s204LedgerFilter: Scenario = {
  ...s200Sessions,
  name: "ledger-filter",
  says: "A long Session's ledger: open pull requests, running subagents and Jobs by default, the finished ones behind All",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, more) },
};
