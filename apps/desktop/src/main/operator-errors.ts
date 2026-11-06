// An error Bridge showed the person, sent to Fleet's Operator: `POST /operator/errors`. Fleet answers 200 whether or not
// the Operator is on, so a refusal here is Fleet unreachable or a body it could not read.

import type { BridgeErrorTaken, Outcome, ReportBridgeError } from "@armada/protocol";
import type { OperatorErrorSent } from "../shared/api/operator-errors";
import { ask } from "./request";

const NOT_CONNECTED: Outcome = {
  ok: false,
  why: "not_connected",
};

export class OperatorErrorsHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async report(error: ReportBridgeError): Promise<OperatorErrorSent> {
    const port = this.port();
    if (port === null)
      return {
        ok: false,
        outcome: NOT_CONNECTED,
      };
    const answer = await ask(port, "POST", "/operator/errors", error);
    return answer.ok === true
      ? {
          ok: true,
          value: answer.body as BridgeErrorTaken,
        }
      : {
          ok: false,
          outcome: answer.outcome,
        };
  }
}
