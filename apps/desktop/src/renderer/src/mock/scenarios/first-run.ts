// A Fleet serving no repository, and a person with a folder and a request: the Cockpit's stage is a prompt, a Session starts on a folder
// with no Manifest, and Armada offers to remember how the folder runs. Over `first-launch`'s state. Mock only: the folder dialog
// answers `~/code/my-app`, and listing it adds a repository with no Manifest. The walk `first-run` plays it.

import type { RepositorySummary } from "@armada/protocol";

import type { Area } from "../../cockpit/deck/areas";
import { FOLDER, firstRunStore } from "../first-run";
import { connected } from "../moment";
import type { Scenario } from "../moment";

/** Areas of the project, standing in for what Fleet reads of a repository it has just been given. */
const AREAS: readonly Area[] = [
  { dir: "src", kids: [["settings", 6, ["Settings.tsx", "Settings.test.tsx", "Toggle.tsx", "index.ts", "settings.css", "storage.ts"]], ["theme", 4, ["tokens.css", "useTheme.ts", "theme.test.ts", "index.ts"]], ["components", 12]] },
  { dir: "public", kids: [], names: ["favicon.ico", "index.html"], files: 2 },
];

export const s300FirstRun: Scenario = {
  name: "first-run",
  says: "Fleet serving no repository: a folder and a request start a Job with no Manifest",
  state: connected([], []),
  behaves: (fleet) => ({
    chooseFolder: async () => FOLDER,
    resolveFolder: async (path) => path,
    // Fleet lists a folder it was given, a Manifest or not.
    addRepository: async (path) => {
      const one: RepositorySummary = { root: path, records_root: `/Users/user/Library/Application Support/Armada/records/${path.split("/").pop()}` };
      const held = fleet.state().holds.repositories ?? [];
      if (!held.some((it) => it.root === path)) fleet.publish({ holds: { ...fleet.state().holds, repositories: [...held, one] } });
      return { state: "located", repository: one };
    },
  }),
  draft: { sessions: () => firstRunStore(), areas: () => AREAS },
};
