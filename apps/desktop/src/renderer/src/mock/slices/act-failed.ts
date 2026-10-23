// A background act's failure: Fleet alone sends it, so the mock has none.

import type { ActFailedApi } from "../../../../shared/api/act-failed";
import type { Slice } from "../fake-context";

export const actFailed: Slice<ActFailedApi, Record<string, never>> = {
  name: "actFailed",
  state: {},
  api: () => ({
    onActFailed: () => () => {},
  }),
};
