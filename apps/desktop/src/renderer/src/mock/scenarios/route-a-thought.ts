// Helm routes a thought into running work. Helm's dock is open on armada, and three repositories are
// served with live Sessions in each: a thought typed in the dock is read against all of them. The
// Session "Sessions list views" holds Sessions-page work, "Invoice PDF layout" is in billing, and
// nothing anywhere is about the merge line. A plain question gets Helm's reply. The walk
// `route-a-thought` plays it. The reading is the mock's own (`mock/thought-reading.ts`), and the words sent on are
// kept in the thread of Helm's Session as Fleet keeps them, the Session being made by the first
// message or thought for the repository (`mock/helm-carry.ts`).

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { FakeSessionsFleet, held, hosted, pullRequest } from "../sessions-fleet";
import { COMMANDS, helmFleetOn } from "../helm-carry";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

/** A repository served beside armada, set up. */
function elsewhere(name: string): RepositorySummary {
  return {
    root: `/Users/user/code/${name}`,
    records_root: `/records/${name}`,
    manifest: { ...repository().manifest!, id: name, repository: name, path: `${name}/armada.yml` },
  };
}

const SERVED = [ARMADA, elsewhere("storefront"), elsewhere("billing")];

const now = () => new Date().toISOString();

/** An attachment of another repository's: `held` writes armada's. */
const of = (manifest: string, attachment: ReturnType<typeof held>) => ({ ...attachment, manifest_id: manifest });

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

/** Helm's reply to a question about a Session, in the Session's own facts and nothing past them. */
const REPLY = [
  "Store migration spike is idle. Its last turn ended on a failed cargo_nextest run.",
  "Expected — cargo nextest run --workspace exits 0",
  "Produced — exit 101 — 3 of 2034 tests failed",
].join("\n\n");

function build(): Scenario {
  const base = holding("route-a-thought", "Helm routes a thought into running work: live Sessions in three repositories, and Helm's dock open on armada");
  const board: Scenario = {
    ...base,
    state: {
      ...base.state,
      holds: { ...base.state.holds, repositories: SERVED, manifests: SERVED.flatMap((one) => (one.manifest === undefined ? [] : [one.manifest])) },
      helm: { state: "pointed", manifestId: "armada" },
    },
  };
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  return {
    ...served,
    // Each mount has a Fleet of its own, so a Helm Session one window made is not there for the next.
    behaves: (handle) => helmFleetOn(served, handle, REPLY),
  };
}

export const s210RouteAThought: Scenario = build();
