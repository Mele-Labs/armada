// Manifest's registration: its members come from `@armada/manifest/fake`, over this fake's handle.

import { manifestApi } from "@armada/manifest/fake";

import type { ManifestApi, ManifestState } from "../../../../shared/api/manifest";
import { MANIFEST_NOTHING_YET } from "../../../../shared/api/manifest";
import type { Slice } from "../fake-context";
import { checking, checkingWithoutRunSheet } from "../checks-fleet";
import { DRIFT_GONE, GH_ISSUE_VIEW, KIT_SERVERS, RUNS, manifesting } from "../manifest-fake";

export const manifest: Slice<ManifestApi, ManifestState> = {
  name: "manifest",
  state: MANIFEST_NOTHING_YET,
  scenarios: () => [manifesting({ alwaysAllowed: [GH_ISSUE_VIEW], drift: DRIFT_GONE, kitServers: KIT_SERVERS, runs: RUNS }), checking(), checkingWithoutRunSheet()],
  api: (_scenario, fleet) => manifestApi(fleet),
};
