// A real Job's thread, the panel header sizes and the questions it put to him, in the new look. Over the `real-thread-header-sizes` scenario.

import { realThreadWalk } from "../real-threads/real-thread";

const walk = realThreadWalk("real-thread-header-sizes", "Header Font Sizes", /Ran /);

export { walk as "real-thread-header-sizes" };
