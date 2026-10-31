// System > Workspaces and the repository picker's Workspaces (armada-cwj). Six repositories are served: store-web and store-api in my-store, which is kept on this Mac, store-docs, store-infra and store-api in platform,
// which is committed in store-docs, and scratch and store-mobile, in none. store-api is in both, and the two set Commands differently. Built over `workspaces-lanes`, whose Workspace and Jobs it keeps. A mock, no Fleet behind it.

import type { RepositorySummary } from "@armada/protocol";

import type { Scenario } from "../moment";
import { workflowsServed } from "../workflows-fleet";
import { MY_STORE, PROPOSALS, STORE_DEFINITIONS, STORE_ENDS } from "../workspaces-lanes-data";
import { workspacesLanes } from "./workspaces-lanes";

const [WEB, API] = workspacesLanes.state.holds.repositories ?? [];

/** A repository served beside the two the base holds: its own Manifest, under a root of its own. */
const another = (id: string, name: string): RepositorySummary => ({
  root: `/Users/user/${name}`,
  records_root: `/Users/user/Library/Application Support/Armada/records/${name}`,
  manifest: { ...WEB!.manifest!, id, repository: name, path: `${name}/armada.yml` },
});

export const REPOSITORIES = [WEB!, API!, another("01M1CNPKTV0018H2M1CXDNBK08", "store-docs"), another("01M1CNPKTV0018H2M1CXDNBK09", "store-infra"), another("01M1CNPKTV0018H2M1CXDNBK0A", "scratch"), another("01M1CNPKTV0018H2M1CXDNBK0B", "store-mobile")];

export const workspacesAndAutonomy: Scenario = {
  ...workspacesLanes,
  name: "workspaces-and-autonomy",
  says: "The repository picker lists my-store and platform with their repositories under them, and System > Workspaces sets how far Armada goes for each kind of act at Everywhere, at each Workspace and at each repository. store-api is in both Workspaces, which set Commands differently",
  // Opens on store-api, which is in both Workspaces.
  state: { ...workspacesLanes.state, repository: API!.root, holds: { ...workspacesLanes.state.holds, repositories: REPOSITORIES } },
  // Workflows are listed under every repository served, so the page draws the editor on whichever is picked.
  behaves: (fleet) => ({
    ...workspacesLanes.behaves!(fleet),
    ...workflowsServed(() => fleet.state().holds.repositories?.find((one) => one.root === fleet.state().repository)?.manifest?.id ?? null, {
      definitions: STORE_DEFINITIONS,
      ends: STORE_ENDS,
      nudges: [],
      served: REPOSITORIES.map((one) => one.manifest!.id),
    }),
    readManifestFile: workspacesLanes.behaves!(fleet).readManifestFile!,
  }),
  draft: {
    ...workspacesLanes.draft,
    workspace: {
      workspace: MY_STORE,
      proposals: PROPOSALS,
      workspaces: {
        served: REPOSITORIES.map((one) => ({ id: one.manifest!.repository, name: one.manifest!.repository })),
        records: [
          { id: "my-store", name: "my-store", repositories: ["store-web", "store-api"], kept: "mac" },
          { id: "platform", name: "platform", repositories: ["store-docs", "store-infra", "store-api"], kept: "repository", home: "store-docs" },
        ],
        layers: {
          everywhere: { tell_agent: "off" },
          "workspace:my-store": { run_command: "asks" },
          "workspace:platform": { run_command: "off" },
        },
      },
    },
  },
};
