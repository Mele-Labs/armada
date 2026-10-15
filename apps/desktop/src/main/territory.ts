// The repositories' directories and the paths each slot's holder has changed, as Fleet answers
// `GET /repository_areas` and `GET /slot_touches`. Each answers for every served repository at once,
// so nothing is named here. Nothing is kept: Fleet keeps its answers, and the renderer decides when to
// ask again.

import type { Outcome, RepositoryAreas, SlotTouches } from "@armada/protocol";
import type { RepositoryAreasRead, SlotTouchesRead } from "../shared/api/territory";
import { ask, type Answer } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

/** A Fleet without these routes answers 404 from the router, with no refusal's body. */
const hasNoRoute = (answer: Answer): boolean =>
  !answer.ok && answer.outcome.ok === false && answer.outcome.why === "transport" && answer.outcome.fault.why === "unanswerable" && answer.outcome.fault.status === 404;

export class TerritoryHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  /** What the routes say, or nothing where Fleet has no such route: no directories, not a refusal. */
  async areas(): Promise<RepositoryAreasRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", "/repository_areas");
    if (hasNoRoute(answer)) return { ok: true, value: { repositories: [] } };
    return answer.ok === true ? { ok: true, value: answer.body as RepositoryAreas } : { ok: false, outcome: answer.outcome };
  }

  async touches(): Promise<SlotTouchesRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "GET", "/slot_touches");
    if (hasNoRoute(answer)) return { ok: true, value: { repositories: [] } };
    return answer.ok === true ? { ok: true, value: answer.body as SlotTouches } : { ok: false, outcome: answer.outcome };
  }
}
