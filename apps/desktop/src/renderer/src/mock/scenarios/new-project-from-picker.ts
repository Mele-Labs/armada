// A new project started from the Cockpit's repository picker, past the first run: the ship at rest, the folder dialog answering ~/Developer, and the folder
// made served with a Manifest so the new Job's header names it. Mock only: main spells out `~` as the home it knows. The walk `new-project-from-picker` plays it.

import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { manifestOf } from "../first-run";
import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship, shipSessionsStarting } from "./cockpit-ship";

const HOME = "/Users/user";

export const newProjectFromPicker: Scenario = {
  ...ship,
  name: "new-project-from-picker",
  says: "The ship at rest: New project in the repository picker makes a folder and opens a Job on it",
  draft: { ...ship.draft, sessions: shipSessionsStarting((root) => (root.startsWith(`${HOME}/Developer/`) ? manifestOf(root) : undefined)) },
  behaves: (fleet) => ({
    ...ship.behaves?.(fleet),
    chooseFolder: async () => `${HOME}/Developer`,
    createProject: async (parent, name) => {
      const root = `${parent.replace(/^~(?=\/|$)/, HOME).replace(/\/$/, "")}/${name}`;
      const id = manifestOf(root);
      const one: RepositorySummary = { root, records_root: `${HOME}/Library/Application Support/Armada/records/${id}`, manifest: { ...repository().manifest!, id, repository: id, path: `${root}/armada.yml` } };
      const held = fleet.state().holds.repositories ?? [];
      if (!held.some((it) => it.root === root)) fleet.publish({ holds: { ...fleet.state().holds, repositories: [...held, one] } });
      return { state: "located", repository: one };
    },
  }),
};
