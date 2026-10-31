// The Job "Dock Icon Electron" as it really went, in the new thread look, for the walk `real-thread-dock-icon`. Over `sessions`.

import type { Scenario } from "../moment";
import { DOCK_ICON_ROWS } from "../real-threads/dock-icon";
import { realThreadScenario } from "../real-threads/real-thread";

export const s341bRealThreadDockIcon: Scenario = realThreadScenario({
  name: "real-thread-dock-icon",
  says: "A real Job's thread, from the larger dock icon to its pull request waiting in the merge queue",
  id: "c307370a-4054-4017-8ace-010739ec623c",
  title: "Dock Icon Electron",
  branch: "armada/session-c307370a",
  slot: 7,
  rows: DOCK_ICON_ROWS,
  asked: ["Tell me when it lands", "Anything for me?", "Thanks"],
});
