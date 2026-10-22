// Write clashes: the wire is answered by Fleet alone, so the mock has none for it. A scenario that
// wants marks carries its list (`draft.clashes`), which `mount.tsx` hands to `ClashesFrom`.

import type { ClashesApi } from "../../../../shared/api/clashes";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const clashes: Slice<ClashesApi, Record<string, never>> = {
  name: "clashes",
  state: {},
  api: () => ({
    getWriteClashes: async () => ({ ok: false, outcome: unanswered("/clashes") }),
    readClashFile: async () => ({ ok: false, outcome: unanswered("/clashes/file") }),
    readSessionFileChange: async () => ({ ok: false, outcome: unanswered("/sessions/file_change") }),
  }),
};
