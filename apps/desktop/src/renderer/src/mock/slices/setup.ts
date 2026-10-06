// Setup's members: the repository scan, the manifest proposals, and adding a repository.

import { repository } from "@armada/screens/src/fixtures/build/base";

import type { SetupApi, SetupState } from "../../../../shared/api/setup";
import { SETUP_NOTHING_YET } from "../../../../shared/api/setup";
import type { Slice } from "../fake-context";
import { SCRATCH, SHEET_READ, settingUp } from "../setup-fleet";

export const setup: Slice<SetupApi, SetupState> = {
  name: "setup",
  state: SETUP_NOTHING_YET,
  scenarios: () => [settingUp({ repositories: [repository(), SCRATCH], sheet: SHEET_READ })],
  api: (_scenario, { refused, unread }) => ({
    readRepositoryScan: async () => refused("/repositories/scan"),
    readManifestProposals: async () => refused("/manifest/proposals"),
    editManifestProposal: async () => unread("/manifest/proposals"),
    writeManifestProposal: async () => unread("/manifest/proposals"),

    chooseFolder: async () => null,
    resolveFolder: async () => null,
    addRepository: async () => unread("/repositories"),
    cloneRepository: async () => unread("/repositories/clone"),
  }),
};
