// A Fleet serving no repository, and a person with a folder and a request: the Cockpit's stage is a prompt, a Session starts on a folder
// with no Manifest, and Armada offers to remember how the folder runs. Over `first-launch`'s state. Mock only: the folder dialog
// answers `~/code/my-app`, and a folder listed, added or made new is given a Manifest so a Job's header names its repository. The walk `first-run` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { Area } from "../../cockpit/deck/areas";
import { FOLDER, firstRunStore, installedMod, manifestOf } from "../first-run";
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
  behaves: (fleet) => {
    // Fleet lists a folder it was given, a Manifest or not.
    const serve = (path: string): RepositorySummary => {
      const id = manifestOf(path);
      // Mock only: Fleet lists a folder with no Manifest without one. A Job's header names its repository only where there is one, so the folder is given the fixture's.
      const one: RepositorySummary = { root: path, records_root: `/Users/user/Library/Application Support/Armada/records/${id}`, manifest: { ...repository().manifest!, id, repository: id, path: `${path}/armada.yml` } };
      const held = fleet.state().holds.repositories ?? [];
      if (!held.some((it) => it.root === path)) fleet.publish({ holds: { ...fleet.state().holds, repositories: [...held, one] } });
      return one;
    };
    return {
      chooseFolder: async () => FOLDER,
      resolveFolder: async (path) => path,
      addRepository: async (path) => ({ state: "located", repository: serve(path) }),
      createProject: async (parent, name) => ({ state: "located", repository: serve(`${parent.replace(/\/$/, "")}/${name}`) }),
      // The mod's install shows the terminal session as a Job.
      installAgentMod: async () => {
        installedMod();
        return { ok: true, value: { result: "installed", said: "Installed armada@armada-local" } };
      },
    };
  },
  draft: { sessions: () => firstRunStore(), areas: () => AREAS },
};
