// A mock Fleet takes the report and keeps nothing: the Operator is not drawn from it here.

import type { OperatorErrorsApi } from "../../../../shared/api/operator-errors";
import type { Slice } from "../fake-context";
import { unanswered } from "../moment";

export const operatorErrors: Slice<OperatorErrorsApi, Record<string, never>> = {
  name: "operatorErrors",
  state: {},
  api: () => ({
    reportBridgeError: async () => ({
      ok: false,
      outcome: unanswered("/operator/errors"),
    }),
  }),
};
