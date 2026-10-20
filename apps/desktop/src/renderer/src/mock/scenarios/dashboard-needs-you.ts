// The Dashboard with a good deal waiting on the owner: a Session waiting on a command, main red with
// nobody on it, and a pull request the line refused. Beside them, work for the Running tab (a Session
// at work, a line in flight) and for Done (an ended Session, what landed).

import { repository } from "@armada/screens/src/fixtures/repository";
import { mergeLines } from "@armada/screens/src/fixtures/merge-line";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import { mainRed } from "../main-red-hub";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const quiet = { state: "idle" } as const;
const said = (id: string, text: string) => [{ id, at: "14:00:00", kind: "message", from: { kind: "agent" }, text } as const];

/** One waiting on a command, one at work, one over. */
const SESSIONS: Session[] = [
  {
    id: "s9",
    title: "Flaky store test",
    turn: quiet,
    lastTurnAt: ago(3),
    rows: [...said("s9-0", "The flaky test is the 200ms sleep in store_test.rs; I replaced it with a wait on the channel."), ...said("s9-1", "The branch is ready to push.")],
    attachments: [],
    asked: { command: "git push --force-with-lease origin fix/flaky-store" },
  },
  { id: "s10", title: "Migration notes", turn: { state: "working" }, lastTurnAt: ago(1), rows: [...said("s10-0", "Listing the migrations since 0042."), ...said("s10-1", "Reading the migrations.")], attachments: [] },
  // A real Session's own question, as it came: three options, each with its line, and the question whole.
  {
    id: "s14",
    title: "Preview Refresh from Bridge",
    turn: quiet,
    lastTurnAt: ago(50),
    rows: said("s14-0", "The preview ran against a build from before the filters landed."),
    attachments: [],
    asked: {
      command: "AskUserQuestion",
      call: "q14",
      questions: [
        {
          question:
            "The preview refresh restarted Bridge on a build from before the filters landed, so the new panel never showed. Fixing it is one line in the restart script. Nothing you see changes either way. What should happen to it?",
          header: "Stale build",
          multi_select: false,
          options: [
            { label: "File an issue (Recommended)", description: "Write it up so it is tracked, and leave the restart script as it is for now." },
            { label: "Fix it now", description: "Change the restart script in this session, run the preview again and push the branch." },
            { label: "Drop it", description: "Leave it. It does not matter enough to track." },
          ],
        },
      ],
    },
  },
  // A Session on the wire's own shape: what it waits on, in the order each began. A page to look at, then a question with no options.
  {
    id: "s15",
    title: "Docs for the pairing screen",
    turn: quiet,
    lastTurnAt: ago(30),
    rows: said("s15-0", "The pairing page is up, and I need one decision before I write its caption."),
    attachments: [],
    waitingFor: [
      { id: "ask:q15", text: "Which of the two pairing codes should the QR carry, the long one or the short?", since: ago(28), source: "agent" },
      { id: "walk:https://git.example/pairing", text: "The pairing screen is ready to look at", since: ago(34), source: "walk", act: { kind: "walk", target: "https://git.example/pairing" } },
    ],
  } as Session,
  // A Session that holds a failing pull request: the call about it sends him to the Session.
  {
    id: "s13",
    title: "Armada Pocket",
    turn: quiet,
    lastTurnAt: ago(25),
    rows: said("s13-0", "The pairing screen is up and the PWA shell caches offline."),
    attachments: [{ kind: "pull_request", number: 1822, title: "Pair a phone with Fleet", branch: "pocket/1998-pwa", address: "https://git.example/armada/pull/1822", checks: { state: "failed", failing: "desktop_test" }, state: "open", auto: false }],
  },
  { id: "s11", title: "Release script", dead: "ended", turn: quiet, lastTurnAt: ago(150), rows: said("s11-1", "Done."), attachments: [] },
];

function build(): Scenario {
  const base = holding("dashboard-needs-you", "The Dashboard with a Session waiting and main red");
  // The fourth moment of main going red: a person merged it by hand, so nobody has the red. Its
  // line is the recorded one beside it, so a turn is in flight and three branches have landed.
  const moment = mainRed(repository().root).later[2]!;
  const red = (moment.mergeLines?.lines ?? [])[0]!;
  // Two more failing pull requests: #1822 is held by a Session, #1823 is a person's with nobody on it.
  const pulls = [
    ...(red.hub?.pull_requests ?? []),
    { number: 1822, title: "Pair a phone with Fleet", branch: "pocket/1998-pwa", url: "https://git.example/armada/pull/1822", ci: "failed" },
    { number: 1823, title: "Bump the lockfile", branch: "chore/bump-lockfile", url: "https://git.example/armada/pull/1823", ci: "failed" },
  ];
  const lines = { lines: [{ ...mergeLines().lines[0]!, hub: { ...red.hub!, pull_requests: pulls } }] };
  return {
    ...base,
    state: { ...base.state, ...moment, repository: repository().root, mergeLines: lines },
    draft: { sessions: () => sessionsStore(SESSIONS) },
  };
}

export const s205DashboardNeedsYou: Scenario = build();
