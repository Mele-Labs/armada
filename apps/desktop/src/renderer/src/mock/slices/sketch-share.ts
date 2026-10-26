// Sharing a sketch: no dialog to open and no disk to write. A scenario or a walk that wants to see what was
// saved supplies its own through `Scenario.behaves`.

import type { SketchShareApi } from "../../../../shared/api/sketch-share";
import type { Slice } from "../fake-context";

export const sketchShare: Slice<SketchShareApi, Record<string, never>> = {
  name: "sketchShare",
  state: {},
  api: () => ({
    saveSketchFile: async (files) => ({ saved: true, name: files[0]!.name }),
    saveDocument: async (request) => ({ saved: true, name: request.name }),
  }),
};
