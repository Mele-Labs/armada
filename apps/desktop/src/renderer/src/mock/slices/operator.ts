// The Operator page's reads and acts, answered harmlessly: no message history, a fixed reply, and a board that cannot
// be moved. A scenario that draws the board sets `operatorBoard` on its state. Mock of Fleet's routes, which main reaches in
// `main/operator.ts`.

import type { OperatorApi, OperatorState } from "../../../../shared/api/operator";
import { OPERATOR_NOTHING_YET } from "../../../../shared/api/operator";
import type { Slice } from "../fake-context";

export const operator: Slice<OperatorApi, OperatorState> = {
  name: "operator",
  state: OPERATOR_NOTHING_YET,
  api: () => ({
    readOperatorTalk: async () => ({
      ok: true,
      talk: {
        lines: [],
      },
    }),
    talkToOperator: async () => ({
      ok: true,
      said: {
        said: "Noted.",
      },
    }),
    moveOnOperatorBoard: async () => ({
      ok: false,
      outcome: {
        ok: false,
        why: "not_connected",
      },
    }),
  }),
};
