// Overview's member: the watch held for the life of the window.

import type { OverviewApi } from "../api";

// Held for the life of the window: a failure here would draw every surface's Fleet panel in trouble.
export const overviewApi = (): OverviewApi => ({ watchOverview: async () => undefined });
