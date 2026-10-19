// A new project as the first start: the folder picker answers ~/Developer, and the Job starts in the folder that was made. Over `first-run`.
// The walk `first-run-new` plays it.

import type { Scenario } from "../moment";
import { s300FirstRun } from "./first-run";

export const s301FirstRunNew: Scenario = {
  ...s300FirstRun,
  name: "first-run-new",
  says: "Fleet serving no repository: a name and a place start a Job in a new folder",
  behaves: (fleet) => ({ ...s300FirstRun.behaves?.(fleet), chooseFolder: async () => "/Users/user/Developer" }),
};
