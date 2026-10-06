// Manifest's members: the checkout's Runs and drift, the manifest file, and Kit's allowlist and servers.

import type { Outcome } from "@armada/protocol";

import type { ManifestApi, ManifestState } from "../../../../shared/api/manifest";
import { MANIFEST_NOTHING_YET } from "../../../../shared/api/manifest";
import type { Slice } from "../fake-context";
import { nothing } from "../fake-context";
import { checking } from "../checks-fleet";
import { KIT_INVENTORY, withAllowed } from "../kit-inventory";
import { DRIFT_GONE, GH_ISSUE_VIEW, KIT_SERVERS, RUNS, manifesting } from "../manifest-fleet";

const OK: Outcome = { ok: true };

export const manifest: Slice<ManifestApi, ManifestState> = {
  name: "manifest",
  state: MANIFEST_NOTHING_YET,
  scenarios: () => [manifesting({ alwaysAllowed: [GH_ISSUE_VIEW], drift: DRIFT_GONE, kitServers: KIT_SERVERS, runs: RUNS }), checking()],
  api: (_scenario, { publish, refused, unread, kit }) => ({
    watchCheckoutRunSheet: async (want) => publish({ checkoutRunSheet: want ? unread("/manifest/runs/sheet") : nothing }),
    observeCheckoutRun: async () => publish({ checkoutRunFollowed: nothing }),
    startCheckoutRun: async () => OK,
    stopCheckoutRun: async () => OK,
    undoCheckoutRun: async () => OK,
    listCheckoutRuns: async () => refused("/manifest/runs"),
    getCheckoutRunOutput: async (runId) => refused(`/manifest/runs/${runId}/output`),
    getCheckoutRunDiff: async (runId) => refused(`/manifest/runs/${runId}/diff`),
    watchManifestDrift: async (want) => publish({ manifestDrift: want ? unread("/manifest/drift") : nothing }),
    startCheckoutVerify: async () => OK,

    readManifestFile: async () => refused("/manifest/file"),
    saveManifestFile: async () => unread("/manifest/file"),
    editManifest: async () => unread("/manifest/edit"),
    readManifestSpend: async () => refused("/manifest/spend"),
    readManifestChecks: async () => refused("/manifest/checks"),

    listRepositoryAllowedCommands: async () => refused("/manifest/allowed-commands"),
    removeRepositoryAllowedCommand: async () => refused("/manifest/allowed-commands"),

    // A scenario that has put nothing in the allowlist keeps the Kit read refused, as it was.
    readKitInventory: async () =>
      kit.get().length === 0 ? refused("/kit/inventory") : { ok: true, setup: withAllowed(KIT_INVENTORY, kit.get()) },
    removeKitAllowedCommand: async (run) => {
      if (!kit.get().some((one) => one.run === run)) return refused("/kit/allowed_commands/remove");
      kit.set(kit.get().filter((one) => one.run !== run));
      return { ok: true, commands: { commands: kit.get() } };
    },
    listKitServers: async () => refused("/kit/servers"),
    addKitServer: async () => refused("/kit/servers"),
    forgetKitServer: async () => refused("/kit/servers"),
    setKitServerReach: async () => refused("/kit/servers"),
    setManifestServerReach: async () => refused("/kit/servers"),
  }),
};
