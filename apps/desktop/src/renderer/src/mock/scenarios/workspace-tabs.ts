// The Workflows page for `api`, which is in two Workspaces: Platform (api and web) and Mobile (api and ios). api keeps a copy of feature of its own, and picking it in the list sets the level the
// page edits at to api's. Platform and Mobile each set how far the rule Mark ready goes alone differently from the default's, so api, which is in both, reads the default's and warns. Built over
// `workspaces-lanes`, whose Jobs it keeps. A mock, no Fleet behind it.

import type { RepositorySummary } from "@armada/protocol";

import type { Scenario } from "../moment";
import { mobilePieces } from "../pieces-seed";
import { workflowsServed } from "../workflows-fleet";
import { STORE_DEFINITIONS, STORE_ENDS } from "../workspaces-lanes-data";
import { workspacesLanes } from "./workspaces-lanes";

const [WEB] = workspacesLanes.state.holds.repositories ?? [];

/** A repository served beside the base's: its own Manifest, under a root of its own. */
const repository = (id: string, name: string): RepositorySummary => ({
  root: `/Users/user/${name}`,
  records_root: `/Users/user/Library/Application Support/Armada/records/${name}`,
  manifest: {
    ...WEB!.manifest!,
    id,
    repository: name,
    path: `${name}/armada.yml`,
  },
});

const API = repository("01M1CNPKTV0018H2M1CXDNBK0C", "api");
const WEB_APP = repository("01M1CNPKTV0018H2M1CXDNBK0D", "web");
const IOS = repository("01M1CNPKTV0018H2M1CXDNBK0E", "ios");
const REPOSITORIES = [
  API,
  WEB_APP,
  IOS,
];

export const workspaceTabs: Scenario = {
  ...workspacesLanes,
  name: "workspace-tabs",
  says: "The Workflows page on api, which is in Platform and in Mobile: api's own copy of feature sets the level. The two set Mark ready differently, so api reads the default's and warns",
  state: {
    ...workspacesLanes.state,
    repository: API.root,
    holds: {
      ...workspacesLanes.state.holds,
      repositories: REPOSITORIES,
    },
  },
  behaves: (fleet) => ({
    ...workspacesLanes.behaves!(fleet),
    ...workflowsServed(
      () =>
        fleet.state().holds.repositories?.find((one) => one.root === fleet.state().repository)
          ?.manifest?.id ?? null,
      {
        definitions: STORE_DEFINITIONS,
        ends: STORE_ENDS,
        served: REPOSITORIES.map((one) => one.manifest!.id),
        holds: API.manifest!.id,
      },
    ),
    readManifestFile: workspacesLanes.behaves!(fleet).readManifestFile!,
  }),
  draft: {
    pieces: mobilePieces(
      {
        id: API.manifest!.id,
        root: API.root,
      },
      {
        id: WEB_APP.manifest!.id,
        root: WEB_APP.root,
      },
      {
        id: IOS.manifest!.id,
        root: IOS.root,
      },
    ),
  },
};
