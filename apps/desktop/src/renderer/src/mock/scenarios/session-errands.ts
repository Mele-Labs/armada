// A Session that waits on the owner for five things, one card each in the Cockpit: a slash command
// for the Session itself, a command he runs, a pull request to approve, a plain line, and a real
// question with options. Only the question carries the standing answers. The walk `session-errands`
// plays it.

import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const SESSION: Session = {
  id: "s20",
  title: "Land the speed fixes",
  turn: { state: "idle" },
  lastTurnAt: ago(2),
  rows: [
    { id: "e1", at: "14:10:00", kind: "message", from: { kind: "you" }, text: "Get the three speed fixes through the merge line." },
    { id: "e2", at: "14:20:00", kind: "message", from: { kind: "agent" }, text: "Three pull requests are open and checks are running." },
  ],
  attachments: [
    { kind: "pull_request", number: 2118, title: "Cache the manifest read", branch: "speed/manifest-cache", address: "https://git.example/armada/pull/2118", checks: { state: "passed" }, state: "open", auto: false },
  ],
  waitingFor: [
    { id: "e-slash", text: "/land 2109 2118 2119 — speed fixes already running in your Fleet", since: ago(9), source: "agent", act: { kind: "run", target: "/land 2109 2118 2119" } },
    { id: "e-shell", text: "Restart onto the preview", since: ago(8), source: "agent", act: { kind: "run", target: "kill $(lsof -ti tcp:5788) && scripts/preview --restart --adopt" } },
    { id: "e-pull", text: "Approve #2118", since: ago(7), source: "agent", act: { kind: "approve_pr", target: "2118" } },
    { id: "e-plain", text: "Which branch should the migration rebase onto", since: ago(6), source: "agent" },
    {
      id: "e-ask",
      text: "Which clock should the store tests use?",
      since: ago(5),
      source: "ask_card",
      act: { kind: "answer", target: "q-20" },
      options: [{ label: "A fake the test sets" }, { label: "The real one, stopped" }],
    },
  ],
};

function build(): Scenario {
  const base = holding("session-errands", "The Cockpit with one Session waiting on a slash command, a shell command, a pull request, a plain line and a question");
  return {
    ...base,
    state: { ...base.state, repository: repository().root },
    draft: { sessions: () => sessionsStore([SESSION]) },
  };
}

export const s210SessionErrands: Scenario = build();
