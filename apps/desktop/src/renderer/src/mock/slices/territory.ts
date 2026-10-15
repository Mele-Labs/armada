// The territory: the wire is answered by Fleet alone, so the mock has none for it. A scenario that
// wants a map's directories carries its own source (`draft.areas`), which `mount.tsx` hands to
// `AreasFrom`.

import type { TerritoryApi } from "../../../../shared/api/territory";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const territory: Slice<TerritoryApi, Record<string, never>> = {
  name: "territory",
  state: {},
  api: () => ({
    getRepositoryAreas: async () => ({ ok: false, outcome: unanswered("/repository_areas") }),
    getSlotTouches: async () => ({ ok: false, outcome: unanswered("/slot_touches") }),
  }),
};
