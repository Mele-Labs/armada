// A Session that waits on the person for a command run once, with the steps that follow it: the store
// Worker's first deploy, as session 2ea2e0dd sent it on 15 Oct, then again with a command in each step, and a third whose commands run from their own keys.
// Beside them a command with no steps, so they read the same. Over the `sessions` scenario. The walk `needs-you-steps-run` plays it.

import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "../moment";
import { s200Sessions } from "./sessions";

const minutesAgo = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const waiting: Session = {
  id: "s16",
  title: "Central store",
  turn: { state: "idle" },
  lastTurnAt: minutesAgo(4),
  rows: [
    { id: "w1", at: "14:02:10", kind: "message", from: { kind: "you" }, text: "Put the store up." },
    { id: "w2", at: "14:40:31", kind: "message", from: { kind: "agent" }, text: "The Worker is built. It deploys once from a machine signed in to Cloudflare." },
  ],
  attachments: [
    { kind: "slot", slot: 2 },
    { kind: "branch", name: "feat/central-store", slot: 2 },
  ],
  waitingFor: [
    {
      id: "deploy-store",
      text: "Deploy the store Worker once, from this machine",
      since: minutesAgo(4),
      source: "agent",
      act: { kind: "run", target: "cd ~/Development/armada/apps/store && pnpm exec wrangler login && pnpm exec wrangler deploy" },
      steps: [
        "Run the command; approve Cloudflare in the browser",
        "Copy the workers.dev address it prints",
        "On each machine, Settings: Account › sign-in app = Client ID, Store › address = that address",
      ],
    },
    {
      id: "deploy-store-by-step",
      text: "Deploy the store Worker, a command a step",
      since: minutesAgo(3),
      source: "agent",
      act: { kind: "run", target: "cd ~/Development/armada/apps/store && pnpm exec wrangler login" },
      steps: [
        "Sign in to Cloudflare and approve it in the browser: `cd ~/Development/armada/apps/store && pnpm exec wrangler login`",
        "Deploy the Worker: `pnpm exec wrangler deploy`",
        "Copy the workers.dev address it prints",
        "Check it answers: `curl -sf https://armada-store.workers.dev/health`",
        "On each machine, Settings: Store › address = that address",
      ],
    },
    {
      id: "beads-setup",
      text: "Put the backlog on this Mac",
      since: minutesAgo(2),
      source: "agent",
      act: { kind: "run", target: "scripts/beads-setup" },
      steps: [
        "Fetch main: `git fetch origin`",
        "Run the store tests, which fail here: `pnpm exec vitest run`",
        "It ends with a count of the issues on this machine",
      ],
    },
    { id: "store-tests", text: "Run the store tests", since: minutesAgo(3), source: "agent", act: { kind: "run", target: "pnpm -C apps/store test" } },
  ],
};

export const s255NeedsYouStepsRun: Scenario = {
  ...s200Sessions,
  name: "needs-you-steps-run",
  says: "A Session waiting on the person for a command with steps, beside a command with none",
  draft: { sessions: (board) => s200Sessions.draft!.sessions!(board, [waiting]) },
};
