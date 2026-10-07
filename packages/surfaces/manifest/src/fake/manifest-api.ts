// Manifest's members as a mock Fleet answers them when no scenario does: nothing read, nothing run,
// except the Kit allowlist a scenario has filled, which Remove acts on.

import type { KitAllowedCommand, Outcome } from "@armada/protocol";

import type { ManifestApi, ManifestState } from "../api";
import { KIT_INVENTORY, withAllowed } from "./kit-inventory";

const OK: Outcome = { ok: true };
const NONE = { state: "none" } as const;

/** What a route with no Fleet behind it gives, the state to publish into, and Kit's allowlist, all of which the app's fake supplies. */
export type ManifestRoutes = {
  publish: (change: Partial<ManifestState>) => void;
  refused: (route: string) => { ok: false; outcome: Outcome };
  unread: (route: string) => { state: "failed"; outcome: Outcome };
  kit: { get: () => KitAllowedCommand[]; set: (allowed: KitAllowedCommand[]) => void };
};

export const manifestApi = ({ publish, refused, unread, kit }: ManifestRoutes): ManifestApi => ({
  watchCheckoutRunSheet: async (want) => publish({ checkoutRunSheet: want ? unread("/manifest/runs/sheet") : NONE }),
  observeCheckoutRun: async () => publish({ checkoutRunFollowed: NONE }),
  startCheckoutRun: async () => OK,
  stopCheckoutRun: async () => OK,
  undoCheckoutRun: async () => OK,
  listCheckoutRuns: async () => refused("/manifest/runs"),
  getCheckoutRunOutput: async (runId) => refused(`/manifest/runs/${runId}/output`),
  getCheckoutRunDiff: async (runId) => refused(`/manifest/runs/${runId}/diff`),
  watchManifestDrift: async (want) => publish({ manifestDrift: want ? unread("/manifest/drift") : NONE }),
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
});
