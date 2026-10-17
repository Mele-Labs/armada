// Two repositories served, on All. The title-bar-repository walk and test play it: the title row names the pick off the Cockpit.

import type { RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { asking, FakeSessionsFleet, held, hosted } from "../sessions-fleet";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

const now = () => new Date().toISOString();
const turn = (state: "idle" | "working") => ({ turn: { state }, mode: "auto" as const, running: true });

const records = () => [
  hosted("01ARMADAASKINGAAAAAAAAAAAA", { title: "Prune the stale branches", last_turn_at: now(), hosted: { ...turn("idle"), asked: asking("git push --force origin prune/stale") } }),
  hosted("01ARMADAWORKINGBBBBBBBBBBB", { title: "Draft the migration notes", last_turn_at: now(), hosted: turn("working") }),
  hosted("01ARMADAIDLECCCCCCCCCCCCCC", { title: "Fix the flaky store test", last_turn_at: now(), attachments: [held("slot", "3")] }),
  hosted("01STOREASKINGDDDDDDDDDDDDD", { manifest_id: "storefront", cwd: "/Users/user/code/storefront", title: "Round the cart totals", last_turn_at: now(), hosted: { ...turn("idle"), asked: asking("git push --force origin cart/rounding") } }),
  hosted("01STOREWORKINGEEEEEEEEEEEE", { manifest_id: "storefront", cwd: "/Users/user/code/storefront", title: "Lay out the invoice PDF", last_turn_at: now(), hosted: turn("working") }),
];

function build(): Scenario {
  const board = onBoard([job("escalated", { id: "01JOBSTOPPED", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" })], {
    repositories: [ARMADA, STOREFRONT],
  });
  const served = new FakeSessionsFleet(records(), {}).scenario(board);
  return {
    ...served,
    name: "sessions-by-repository",
    says: "The Sessions list on All repositories, a band for each of two, and then one picked",
    behaves: (handle) => new FakeSessionsFleet(records(), {}).scenario(board).behaves!(handle),
  };
}

export const s244SessionsByRepository: Scenario = build();
