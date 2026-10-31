// A real Job's thread, the Workspaces design doc and his answers, in the new look. Over the `real-thread-workspaces-doc` scenario.

import { realThreadWalk } from "../real-threads/real-thread";

const walk = realThreadWalk("real-thread-workspaces-doc", "Workspace Autonomy Defaults", /Ran /);

export { walk as "real-thread-workspaces-doc" };
