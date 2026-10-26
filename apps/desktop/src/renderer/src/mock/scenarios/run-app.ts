// Running an app from the Cockpit without a Job: a play glyph on each repository's heading runs it on main, a caret lists its saved runs, and
// the same pair in a Job's header runs the Job's own bay. armada has four saved runs and a live Job; auto-ynab has none, so its play glyph asks
// first, and Fleet drafts what it finds there: nothing installed, so `npm ci` in `frontend` and a `.venv` from `requirements.txt` at the root,
// and a frontend that proxies `/api` to the backend on 8731, a port a leased backend no longer answers on. `npm ci` drops its connection the
// first time. The walk `run-app` plays it.

import type { RepositorySummary } from "@armada/protocol";
import type { RunDraft, RunLink, RunSetupSpec, SavedRun } from "@armada/components";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import type { RunsSeed } from "../runs";
import { sessionsStore } from "../sessions/script";

const ARMADA = repository();
/** A repository nobody wrote an `armada.yml` for, so a run saved there is kept on this Mac. */
const AUTO_YNAB: RepositorySummary = { root: "/Users/user/code/auto-ynab", records_root: "/Users/user/Library/Application Support/Armada/records/auto-ynab", implicit_id: "auto-ynab-6d2e" };

const RUNS: readonly SavedRun[] = [
  { id: "mock-bridge", name: "Mock Bridge", kind: "browser", processes: [{ id: "mock", name: "mock", folder: "apps/desktop", command: "npx vite --config vite.mock.config.ts --port ${port} --strictPort" }] },
  { id: "storybook", name: "Storybook", kind: "browser", processes: [{ id: "storybook", name: "storybook", folder: "packages/components", command: "pnpm exec storybook dev -p ${port} --no-open --ci" }] },
  { id: "bridge-scratch", name: "Bridge dev, scratch Fleet", kind: "app", processes: [{ id: "bridge", name: "bridge", folder: "apps/desktop", command: "pnpm dev:scratch" }] },
  { id: "bridge-mine", name: "Bridge dev, my Fleet", kind: "app", processes: [{ id: "bridge", name: "bridge", folder: "apps/desktop", command: "pnpm dev" }] },
];

const SETUP: readonly RunSetupSpec[] = [
  { id: "frontend-deps", name: "frontend packages", folder: "frontend", command: "npm ci" },
  { id: "venv", name: ".venv", folder: ".", command: "python3 -m venv .venv && .venv/bin/pip install -r requirements.txt" },
];

const FRONTEND_EXPECTS_BACKEND: RunLink = { from: "frontend", to: "backend", port: 8731, found: "frontend/vite.config.js" };

/** What Fleet drafts for auto-ynab from the commands Jobs ran there and from its files. */
const DRAFT: RunDraft = {
  root: "~/Development/auto-ynab",
  savedIn: "this Mac",
  setup: SETUP,
  links: [FRONTEND_EXPECTS_BACKEND],
  processes: [
    { id: "backend", name: "backend", folder: ".", command: "uvicorn backend.app.main:app --port ${port}" },
    { id: "frontend", name: "frontend", folder: "frontend", command: "npx vite --port ${port} --strictPort" },
  ],
};

const SEED: RunsSeed = {
  saved: { [ARMADA.root]: { runs: RUNS, last: "mock-bridge" }, [AUTO_YNAB.root]: { runs: [] } },
  drafts: { [AUTO_YNAB.root]: DRAFT },
  failsOnce: "frontend-deps",
};

const JOB: Session = {
  id: "7c3d9a10-2b45-4e6f-8a91-0d1e2f3a4b5c",
  title: "Pin the store clock",
  manifestId: ARMADA.manifest!.id,
  cwd: ARMADA.root,
  turn: { state: "working" },
  standing: "working",
  rows: [],
  attachments: [
    { kind: "slot", slot: 4 },
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
  ],
};

function build(): Scenario {
  const base = holding("run-app", "The Cockpit with armada and auto-ynab, nothing installed in auto-ynab, whose npm ci fails the first time it runs", [ARMADA, AUTO_YNAB]);
  return {
    ...base,
    state: { ...base.state, repository: null },
    draft: { runs: SEED, sessions: () => sessionsStore([JOB], [], new Map(), []) },
  };
}

export const runApp: Scenario = build();
