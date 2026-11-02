// Workflows on All with two repositories, for the walk `workflows-empty-repository`: harbor keeps its own workflow copies and lighthouse keeps none. Each
// group holds everything that runs in its repository, Armada's or Yours where it keeps no copy, each with a switch that is that repository's. A mock, no Fleet behind it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { manifesting } from "../manifest-fake";
import { connected } from "../moment";
import type { Scenario } from "../moment";
import { builtInPieces } from "../pieces-seed";
import { workflowsServed } from "../workflows-fleet";

const BASE = repository();

/** Harbor takes the first repository's Manifest id, under which `workflows-managed.ts` keeps its repository copies. */
const HARBOR: RepositorySummary = {
  ...BASE,
  root: "/Users/user/harbor",
  records_root: "/Users/user/Library/Application Support/Armada/records/harbor",
  manifest: {
    ...BASE.manifest!,
    repository: "harbor",
    path: "harbor/armada.yml",
  },
};

const LIGHTHOUSE: RepositorySummary = {
  root: "/Users/user/lighthouse",
  records_root: "/Users/user/Library/Application Support/Armada/records/lighthouse",
  manifest: {
    ...BASE.manifest!,
    id: "01M1CNPKTV0018H2M1CXDNBK0L",
    repository: "lighthouse",
    path: "lighthouse/armada.yml",
  },
};

const REPOSITORIES = [
  HARBOR,
  LIGHTHOUSE,
];

const state = connected([], REPOSITORIES);

export const workflowsEmptyRepository: Scenario = {
  name: "workflows-empty-repository",
  says: "All repositories picked: harbor keeps its own workflow copies, lighthouse keeps none and lists what runs there",
  state: {
    ...state,
    repository: null,
  },
  behaves: (fleet) => ({
    ...workflowsServed(
      () =>
        fleet.state().holds.repositories?.find((one) => one.root === fleet.state().repository)
          ?.manifest?.id ?? null,
      {
        served: REPOSITORIES.map((one) => one.manifest!.id),
      },
    ),
    readManifestFile: manifesting().behaves?.(fleet).readManifestFile!,
  }),
  draft: {
    pieces: builtInPieces(),
  },
};
