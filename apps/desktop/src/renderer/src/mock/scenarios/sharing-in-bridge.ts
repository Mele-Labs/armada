// One repository, acme/widgets, with a Job open in it holding a bay, for the walk `sharing-in-bridge`.
// The owner keeps a tile of his own for it, Run tests running `make test-fast`.
// The repository commits two: Run tests running `make test`, which his hides, and a teammate's Lint running `make lint`, not yet approved on this Mac.
// A package pasted on the window holds a teammate's Deploy preview tile for acme/widgets and a review tool kept for a repository this Fleet does not serve, which waits for one to be chosen.
// The clipboard is the walk's own. `docs/design/who-sees-each-piece-of-configuration.md`.

import type { FlightControls, PackagePiece, PieceAdded, RepositorySummary, UnboundPiece } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import { useWalkClipboard } from "../sketch-share-view";
import { PACKAGE } from "../sharing-package";

const ID = "acme-widgets";
const ROOT = "/Users/user/widgets";
const RECORDS = "/Users/user/Library/Application Support/Armada/records/acme-widgets";
const WIDGETS: RepositorySummary = {
  root: ROOT,
  records_root: RECORDS,
  manifest: { ...repository().manifest!, id: ID, repository: "acme/widgets", path: "widgets/armada.yml", records_root: RECORDS },
};

const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();
const clock = (seconds: number): string => new Date(Date.now() - seconds * 1000).toTimeString().slice(0, 8);

const SLOT = 3;
const BRANCH = "mock/widget-tests";

const JOB: Session = {
  id: "sw1",
  title: "Speed up the widget tests",
  manifestId: ID,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  standing: "idle",
  lastTurnAt: ago(2),
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 1 } },
  rows: [{ id: "sw1-u", at: clock(600), kind: "message", from: { kind: "you" }, text: "Split the slow widget specs so the suite runs in under a minute." }],
  attachments: [
    { kind: "slot", slot: SLOT },
    { kind: "branch", name: BRANCH, slot: SLOT },
  ],
};

const YOURS: FlightControls = { version: 1, job: [{ id: "run-tests", kind: "shell", name: "Run tests", text: "make test-fast" }], deck: [] };

const COMMITTED: FlightControls = {
  version: 1,
  job: [
    { id: "run-tests", kind: "shell", name: "Run tests", text: "make test" },
    { id: "lint", kind: "shell", name: "Lint", text: "make lint" },
  ],
  deck: [],
};

const PIECES: PackagePiece[] = [
  { kind: "tile", id: "deploy-preview", name: "Deploy preview", list: "job", action: "shell", commands: ["make preview"], remote: "acme/widgets", repository: ROOT, exists: false },
  { kind: "review_tool", id: "migration-safety", name: "Migration safety", commands: [], remote: "acme/gadgets", exists: false },
];

const WAITING: UnboundPiece = { unbound_id: "u-migration-safety", kind: "review_tool", id: "migration-safety", name: "Migration safety", commands: [], remote: "acme/gadgets" };

function build(): Scenario {
  const base = holding("sharing-in-bridge", "acme/widgets with a Job open: a tile of yours that hides the repository's, Commit, Share and Copy on it, a pasted package to Add, a teammate's tile that asks before it runs, and where its pieces are kept", [WIDGETS]);
  let unbound: UnboundPiece[] = [];
  return {
    ...base,
    state: { ...base.state, repository: null },
    held: {
      slots: [
        {
          manifest_id: ID,
          slot: SLOT,
          path: `${ROOT}/.armada/slots/slot-${SLOT}`,
          base: "main",
          warm: true,
          behind: 0,
          held: { state: "hosted_session", session_id: JOB.id, session_title: JOB.title ?? JOB.id },
          branch: BRANCH,
          since: ago(10),
        },
      ],
    },
    draft: {
      sessions: (_control, more = []) => sessionsStore([...more, JOB], [], new Map(), []),
      repositoryTiles: { [ROOT]: YOURS },
      committedTiles: { [ROOT]: COMMITTED },
    },
    behaves: () => {
      useWalkClipboard();
      return {
        commitToRepository: async (body) => ({
          ok: true,
          value: { repository: body.repository, session_id: "sw-commit", branch: "armada/commit-run-tests", paths: [".armada/flight-controls.toml"], commit: "4f2c9a1", pull_request: "https://git.example/acme/widgets/pull/42" },
        }),
        readPackage: async (text) => (text === PACKAGE ? { ok: true, value: { pieces: PIECES } } : { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.share_malformed", message: "This is not an Armada package", run_id: "run-mock", fields: {}, chain: [] } } }),
        addPackage: async () => {
          unbound = [WAITING];
          const added: PieceAdded[] = [
            { kind: "tile", id: "deploy-preview", name: "Deploy preview", list: "job", remote: "acme/widgets", repository: ROOT, landed: "added" },
            { kind: "review_tool", id: "migration-safety", name: "Migration safety", remote: "acme/gadgets", landed: "unbound", unbound_id: WAITING.unbound_id },
          ];
          return { ok: true, value: { pieces: added } };
        },
        listUnboundPieces: async () => ({ ok: true, value: { pieces: [...unbound] } }),
        bindUnboundPiece: async (body) => {
          unbound = unbound.filter((one) => one.unbound_id !== body.unbound_id);
          return { ok: true, value: { kind: "review_tool", id: "migration-safety", name: "Migration safety", repository: body.repository, landed: "added" } };
        },
      };
    },
  };
}

export const sharingInBridge: Scenario = build();
