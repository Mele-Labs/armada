// Setup's members as a mock Fleet answers them when no scenario does: nothing read, no folder chosen.

import type { Outcome } from "@armada/protocol";

import type { SetupApi } from "../api";

/** The two refusals a route with no Fleet behind it gives, which the app's fake supplies. */
export type SetupRoutes = {
  refused: (route: string) => { ok: false; outcome: Outcome };
  unread: (route: string) => { state: "failed"; outcome: Outcome };
};

export const setupApi = ({ refused, unread }: SetupRoutes): SetupApi => ({
  readRepositoryScan: async () => refused("/repositories/scan"),
  readManifestProposals: async () => refused("/manifest/proposals"),
  editManifestProposal: async () => unread("/manifest/proposals"),
  writeManifestProposal: async () => unread("/manifest/proposals"),

  chooseFolder: async () => null,
  resolveFolder: async () => null,
  addRepository: async () => unread("/repositories"),
  cloneRepository: async () => unread("/repositories/clone"),
});
