// Setup and Locate: the proposals for a repository's Manifest, and adding or cloning a repository.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type { LocateAnswer } from "@armada/screens/src/locate-reads";
import type {
  EditManifestProposal,
  WriteManifestProposal,
  RepositorySummary,
} from "@armada/protocol";
import type {
  ManifestProposalsRead,
  ProposalAnswer,
  RepositoryScanRead,
} from "@armada/screens/src/setup-reads";

export type SetupApi = {
  /** Scan: every workspace in the checkout, read-only. No path crosses. */
  readRepositoryScan: () => Promise<RepositoryScanRead>;
  /** One proposal per workspace, with every edit Fleet holds for it applied. */
  readManifestProposals: () => Promise<ManifestProposalsRead>;
  /** One edit to one proposal, held in Fleet's memory. **Writes nothing.** */
  editManifestProposal: (body: EditManifestProposal) => Promise<ProposalAnswer>;
  /** Create one workspace's `armada.yml`. **Never over a file already there**, and stages nothing. */
  writeManifestProposal: (body: WriteManifestProposal) => Promise<ProposalAnswer>;
  /** The OS's folder dialog, over this window. `null` where the person cancelled it. */
  chooseFolder: () => Promise<string | null>;
  /** A clone parent as Fleet will canonicalise it, or `null` where it is not a folder here. */
  resolveFolder: (path: string) => Promise<string | null>;
  /** Serve a folder, by its absolute path. Listed before this answers; the window picks it. */
  addRepository: (path: string) => Promise<LocateAnswer>;
  /** Clone `url` into a new folder under `parent`, then serve and list it. **Waits past Fleet's ten minutes.** */
  cloneRepository: (url: string, parent: string) => Promise<LocateAnswer>;
};

export type SetupState = {
  /**
   * The last clone Fleet finished serving, and when. Every window receives it so a clone that
   * outlasts its dialog is announced wherever the person is (#926). It moves no pick.
   */
  located: { repository: RepositorySummary; at: number } | null;
};

export const SETUP_NOTHING_YET: SetupState = {
  located: null,
};

export const SETUP_CHANNELS = {
  // Setup: four entries, one per operation, on `readManifestFile`'s terms.
  readRepositoryScan: "bridge:read-repository-scan",
  readManifestProposals: "bridge:read-manifest-proposals",
  editManifestProposal: "bridge:edit-manifest-proposal",
  writeManifestProposal: "bridge:write-manifest-proposal",
  // Locate: the OS folder dialog, and a repository added or cloned. Main asks Fleet; the renderer names paths.
  chooseFolder: "bridge:choose-folder",
  resolveFolder: "bridge:resolve-folder",
  addRepository: "bridge:add-repository",
  cloneRepository: "bridge:clone-repository",
} as const;
