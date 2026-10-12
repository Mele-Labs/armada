// Helm routes a thought into running work. Helm's dock is open on armada, and three repositories are
// served with live Sessions and running Jobs in each: a thought typed in the dock is read against
// all of them. The Session "Sessions list views" holds Sessions-page work, "Retry failed invoice
// charges with backoff" is running in billing, and nothing anywhere is about the merge line. Job 12
// stopped on a gate, so a plain question about it gets Helm's reply. The walk `route-a-thought`
// plays it. The reading is the mock's own (`mock/thought-reading.ts`), and the words sent on are
// kept in Helm's thread as Fleet keeps them.

import type { HelmThreadItem, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { escalatedGateFailure } from "@armada/screens/src/fixtures/build/escalated";
import { featureRunning } from "@armada/jobs/fake";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { asRow, holding } from "../holding";
import type { FleetHandle, Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted, pullRequest } from "../sessions-fleet";
import { readAfterPause } from "../thought-reading";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

/** A repository served beside armada, set up, with nothing of its own on the Board. */
function elsewhere(name: string): RepositorySummary {
  return {
    root: `/Users/user/code/${name}`,
    records_root: `/records/${name}`,
    manifest: { ...repository().manifest!, id: name, repository: name, path: `${name}/armada.yml` },
  };
}

const SERVED = [ARMADA, elsewhere("storefront"), elsewhere("billing")];

/** The fixture's Job, moved onto a repository and off the cleared state the recording ended in. */
function owned(fixture: JobFixture, manifest: string): JobFixture {
  const { reclaimed_at: _cleared, ...job } = { ...fixture.job, owner_manifest_id: manifest };
  return { ...fixture, job };
}

const jobs = [
  owned(asRow(featureRunning(), 71, "pin-the-store-clock", "Pin the Bridge store clock"), "armada"),
  owned(asRow(featureRunning(), 73, "fold-notification-routes", "Fold the two notification routes into one"), "armada"),
  owned(asRow(featureRunning(), 54, "retry-failed-charges", "Retry failed invoice charges with backoff"), "billing"),
  owned(asRow(escalatedGateFailure(), 12, "drone-count", "Stop the Drone count drifting"), "armada"),
];

const now = () => new Date().toISOString();

/** An attachment of another repository's: `held` writes armada's. */
const of = (manifest: string, attachment: ReturnType<typeof held>) => ({ ...attachment, manifest_id: manifest });

/** What `/` offers in a message box: the agent's own skills and commands, as its init line names them. */
const COMMANDS = ["review", "simplify", "security-review", "init", "compact", "clear", "model"];

const records = () => [
  hosted("01LISTVIEWSAAAAAAAAAAAAAAA", {
    title: "Sessions list views",
    last_turn_at: now(),
    hosted: { turn: { state: "working" }, mode: "auto", running: true, commands: COMMANDS },
    attachments: [
      held("slot", "3"),
      held("branch", "sessions/list-views", { slot: "3" }),
      pullRequest(1851, {
        title: "Sessions page: Active, Quiet and Ended views, with a status for each",
        branch: "sessions/list-views",
        state: "draft",
        checks: "pending",
      }),
    ],
  }),
  hosted("01RELEASENOTESBBBBBBBBBBBB", {
    title: "Release notes script",
    last_turn_at: now(),
    attachments: [held("slot", "5"), held("branch", "rel/notes-script", { slot: "5" }), pullRequest(1847, { title: "Release notes from merged pull requests", branch: "rel/notes-script" })],
  }),
  hosted("01MIGRATIONSCCCCCCCCCCCCCC", {
    title: "Store migration spike",
    last_turn_at: now(),
    attachments: [held("slot", "7"), held("branch", "spike/store-migrations", { slot: "7" })],
  }),
  hosted("01CARTROUNDINGDDDDDDDDDDDD", {
    title: "Cart totals rounding",
    manifest_id: "storefront",
    cwd: "/Users/user/code/storefront",
    last_turn_at: now(),
    attachments: [of("storefront", held("slot", "4")), of("storefront", held("branch", "fix/cart-rounding", { slot: "4" }))],
  }),
  hosted("01INVOICEPDFEEEEEEEEEEEEEE", {
    title: "Invoice PDF layout",
    manifest_id: "billing",
    cwd: "/Users/user/code/billing",
    last_turn_at: now(),
    attachments: [of("billing", held("slot", "6")), of("billing", held("branch", "invoice/pdf-layout", { slot: "6" }))],
  }),
];

/** Helm's reply about Job 12, in the fixture's own facts and nothing past them. */
const REPLY = [
  "Job 12 stopped on step 4 of 6, Regression check. cargo_nextest failed on all three attempts.",
  "Expected — cargo nextest run --workspace exits 0",
  "Produced — exit 101 — 3 of 2034 tests failed",
].join("\n\n");

/** Items added to Helm's thread, after the ones it holds: each takes the next id. */
function appended(
  handle: FleetHandle,
  manifestId: string,
  ...items: ((id: string) => HelmThreadItem)[]
): void {
  const held = handle.state().helm;
  const was = held.state === "open" ? held.items : [];
  handle.publish({
    helm: {
      state: "open",
      manifestId,
      replying: false,
      skipped: 0,
      missed: 0,
      items: [...was, ...items.map((item, at) => item(String(was.length + at)))],
    },
  });
}

function build(): Scenario {
  const base = holding(
    "route-a-thought",
    "Helm routes a thought into running work: live Sessions and running Jobs in three repositories, and Helm's dock open on armada",
    jobs,
  );
  const board: Scenario = {
    ...base,
    state: {
      ...base.state,
      holds: { ...base.state.holds, repositories: SERVED, manifests: SERVED.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest])) },
      helm: { state: "open", manifestId: "armada", replying: false, skipped: 0, missed: 0, items: [] },
    },
  };
  const fleet = new FakeSessionsFleet(records(), {});
  const served = fleet.scenario(board);
  return {
    ...served,
    behaves: (handle) => ({
      ...served.behaves!(handle),
      // Fleet's reading of a thought: the mock's keyword scorer over this scenario's own Sessions and Jobs.
      readThought: (text) => {
        const state = handle.state();
        return readAfterPause(text, { sessions: state.sessions.state === "read" ? state.sessions.sessions : [], jobs: state.jobs });
      },
      // Fleet keeps the row for words sent on, after the fact, in the thread that was asked about.
      recordThoughtSent: async (manifestId, text, target) => {
        appended(handle, manifestId, (id) => ({ kind: "sent", id, ts: now(), text, target }));
        return { ok: true };
      },
      // Helm's reply is Fleet's to write; the mock gives the one answer the walk asks for.
      askHelm: async (text) => {
        const at = now();
        appended(
          handle,
          "armada",
          (id) => ({ kind: "asked", id, ts: at, text }),
          (id) => ({ kind: "row", id, turn: { ts: at, seq: Number(id), by: "drone", saw: { event: "said", text: REPLY } } }),
          (id) => ({ kind: "row", id, turn: { ts: at, seq: Number(id), by: "drone", saw: { event: "ended", turns: 2, cost_micros: 14_200, refusals: 0 } } }),
        );
        return { ok: true };
      },
    }),
  };
}

export const s210RouteAThought: Scenario = build();
