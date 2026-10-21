// A Job holding a pull request in the merge line with its checks running, and a teammate's beside it that no Job holds.
// Refresh on either chip reads the forge now; #2290 reads failed. The walks `pull-chip-refresh` and `pull-chip-open-job` play it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet } from "../sessions-fleet";
import { configFailed, configLine, configSessions, configThreads } from "../sessions/pull-chip-refresh";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

/** A Fleet of its own each time, so a walk told twice starts from checks running. */
export function configFleet(): FakeSessionsFleet {
  const fleet = new FakeSessionsFleet(configSessions(), configThreads());
  fleet.forge = configFailed();
  return fleet;
}

function lined(): Scenario {
  const board = onBoard({ repositories: [ARMADA], picked: ARMADA.root });
  return { ...board, state: { ...board.state, mergeLines: { lines: [configLine()] } } };
}

/** The scenario served by `fleet`, for a test that reads what it was asked. */
export const configScenario = (fleet: FakeSessionsFleet): Scenario => fleet.scenario(lined());

function build(): Scenario {
  return {
    ...configScenario(configFleet()),
    name: "pull-chip-refresh",
    says: "A Job's pull request in the merge line with its checks running, which Refresh reads failed, and a teammate's no Job holds",
    behaves: (handle) => configScenario(configFleet()).behaves!(handle),
  };
}

export const pullChipRefresh: Scenario = build();
