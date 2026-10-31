// A real Job's thread, the dock icon fix, in the new look. Over the `real-thread-dock-icon` scenario.

import { realThreadWalk } from "../real-threads/real-thread";

const walk = realThreadWalk("real-thread-dock-icon", "Dock Icon Electron", /Ran /);

export { walk as "real-thread-dock-icon" };
