// A Session that waits on the person for a procedure: a page to register an app on, and the steps to
// follow there. Beside it an item of each ordinary kind, so the difference shows. Over the `sessions`
// scenario. The walk `needs-you-steps` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const waiting: Session = {
  id: "s15",
  title: "Add single sign-on",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(3),
  rows: [
    { id: "w1", at: "10:08:40", kind: "message", from: { kind: "you" }, text: "Add single sign-on." },
    { id: "w2", at: "10:12:03", kind: "message", from: { kind: "agent" }, text: "The code is in. It needs an OAuth app registered with the provider before it can run." },
  ],
  attachments: [
    { kind: "slot", slot: 4 },
    { kind: "branch", name: "feat/single-sign-on", slot: 4 },
  ],
  waitingFor: [
    {
      id: "st1",
      text: "Register the OAuth app",
      since: minutesAgo(3),
      source: "agent",
      act: { kind: "walk", target: "https://auth.example.com/apps/new" },
      steps: [
        "Name it Armada",
        "Any homepage URL and callback URL",
        "Tick Enable Device Flow",
        "Upload a logo",
        "Press Register application",
        "Copy the Client ID and send it here",
      ],
    },
    { id: "st2", text: "Run the sign in tests on your machine", since: minutesAgo(2), source: "agent", act: { kind: "run", target: "pnpm -C apps/desktop test sign-in" } },
    { id: "st3", text: "Keep the old token store or drop it?", since: minutesAgo(2), source: "agent" },
  ],
};

export const s207NeedsYouSteps: Scenario = {
  ...s200Sessions,
  name: "needs-you-steps",
  says: "A Session waiting on the person for a procedure with steps, beside a command and a question",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [waiting]) },
};
