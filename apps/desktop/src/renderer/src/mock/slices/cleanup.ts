// Cleanup's registration: its members come from `@armada/cleanup/fake`, over this fake's handle.

import { cleanupApi } from "@armada/cleanup/fake";

import type { CleanupApi, CleanupState } from "../../../../shared/api/cleanup";
import { CLEANUP_NOTHING_YET } from "../../../../shared/api/cleanup";
import type { Slice } from "../fake-context";
import { proposingRow } from "../fake-context";

export const cleanup: Slice<CleanupApi, CleanupState> = {
  name: "cleanup",
  state: CLEANUP_NOTHING_YET,
  api: (_scenario, fleet) => cleanupApi({ ...fleet, proposingRow }),
};
