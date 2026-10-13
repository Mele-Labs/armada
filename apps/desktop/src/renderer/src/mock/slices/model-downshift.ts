// The default model's step down: the wire is answered by Fleet alone, so the mock has none for it. The offer
// the `plan-downshift` walk plays is `mock/downshift.ts`, a source of its own over the plan's windows.

import type { ModelDownshiftApi } from "../../../../shared/api/model-downshift";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const modelDownshift: Slice<ModelDownshiftApi, Record<string, never>> = {
  name: "modelDownshift",
  state: {},
  api: () => ({
    getModelDownshift: async () => ({ ok: false, outcome: unanswered("/model_downshift") }),
    acceptDownshift: async () => ({ ok: false, outcome: unanswered("/model_downshift/accept") }),
    restoreDownshift: async () => ({ ok: false, outcome: unanswered("/model_downshift/restore") }),
    keepDownshift: async () => ({ ok: false, outcome: unanswered("/model_downshift/keep") }),
    settleDownshift: async () => ({ ok: false, outcome: unanswered("/model_downshift/settle") }),
    onModelDownshiftChanged: () => () => {},
  }),
};
