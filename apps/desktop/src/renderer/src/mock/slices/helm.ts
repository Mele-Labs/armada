// Helm's registration: its member and its moment come from `@armada/helm/fake`.

import { helmApi, talking } from "@armada/helm/fake";

import type { BridgeApi } from "../../../../shared/api";
import type { HelmApi, HelmState } from "../../../../shared/api/helm";
import { HELM_NOTHING_YET } from "../../../../shared/api/helm";
import { NOTHING_YET } from "../../../../shared/bridge";
import type { BridgeState } from "../../../../shared/bridge";
import type { Slice } from "../fake-context";

export const helm: Slice<HelmApi, HelmState> = {
  name: "helm",
  state: HELM_NOTHING_YET,
  scenarios: () => [talking<BridgeState, BridgeApi>(NOTHING_YET)],
  api: (_scenario, fleet) => helmApi(fleet),
};
