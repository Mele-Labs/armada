// The same Cockpit as `run-an-app`, with a link found in auto-ynab's files: its frontend proxies `/api` to the backend on 8731, a port a
// leased backend no longer answers on. The confirm shows the link with Fix with a Job and Pin the port, and a run saved without acting on it
// is refused a few seconds after it is up, so the panel shows the link too. The walk `run-link` plays it.

import type { RunLink } from "@armada/components";

import type { Scenario } from "../moment";
import type { RunsSeed } from "../runs";
import { runAnApp } from "./run-an-app";

const FRONTEND_EXPECTS_BACKEND: RunLink = { from: "frontend", to: "backend", port: 8731, found: "frontend/vite.config.js" };

function build(): Scenario {
  const seed = runAnApp.draft?.runs as RunsSeed;
  const drafts = Object.fromEntries(Object.entries(seed.drafts).map(([root, draft]) => [root, { ...draft, links: [FRONTEND_EXPECTS_BACKEND] }]));
  return {
    ...runAnApp,
    name: "run-link",
    says: "The Cockpit with auto-ynab, whose frontend expects its backend on a fixed port",
    draft: { ...runAnApp.draft, runs: { ...seed, drafts } },
  };
}

export const runLink: Scenario = build();
