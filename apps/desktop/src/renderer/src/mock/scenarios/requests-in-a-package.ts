// One repository, acme/widgets, with a Job open in it, for the walk `requests-in-a-package`.
// A package pasted on the window holds one piece a teammate sent: the Requests kept for acme/widgets, a Store API collection and a test environment.
// Any pasted text naming a `requests` piece reads back that one piece, so the walk owns the text it pastes.

import type { PackagePiece, PieceAdded, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { sessionsStore } from "../sessions/script";
import { useWalkClipboard } from "../sketch-share-view";

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
const BRANCH = "mock/store-api";

const JOB: Session = {
  id: "rq1",
  title: "Check the Store API",
  manifestId: ID,
  model: "opus",
  effort: "high",
  mode: "auto",
  turn: { state: "idle" },
  standing: "idle",
  lastTurnAt: ago(2),
  work: { workflow: { name: "Feature", steps: ["Plan the change", "Implement", "Review the change"], at: 1 } },
  rows: [{ id: "rq1-u", at: clock(600), kind: "message", from: { kind: "you" }, text: "Call the Store API on test and say what comes back." }],
  attachments: [
    { kind: "slot", slot: SLOT },
    { kind: "branch", name: BRANCH, slot: SLOT },
  ],
};

const PIECES: PackagePiece[] = [{ kind: "requests", id: "requests", name: "Store API, test", commands: [], remote: "acme/widgets", repository: ROOT, exists: false }];

function build(): Scenario {
  const base = holding("requests-in-a-package", "acme/widgets with a Job open and a pasted package to Add: one Requests piece, the Store API collection and the test environment", [WIDGETS]);
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
    draft: { sessions: (_control, more = []) => sessionsStore([...more, JOB], [], new Map(), []) },
    behaves: () => {
      useWalkClipboard();
      return {
        readPackage: async (text) =>
          text.includes('kind = "requests"')
            ? { ok: true, value: { pieces: PIECES } }
            : { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.share_malformed", message: "This is not an Armada package", run_id: "run-mock", fields: {}, chain: [] } } },
        addPackage: async () => {
          const added: PieceAdded[] = [{ kind: "requests", id: "requests", name: "Store API, test", remote: "acme/widgets", repository: ROOT, landed: "added" }];
          return { ok: true, value: { pieces: added } };
        },
      };
    },
  };
}

export const requestsInAPackage: Scenario = build();
