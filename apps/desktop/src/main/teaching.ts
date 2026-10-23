// Teaching: read a person's words into a proposal, keep one, and list the patterns Bridge links. Nothing is held
// here; `read` is a model call inside Fleet, so it waits as one does.

import type { Outcome, ReadTeaching, SaveTeaching, SavedTeaching, TeachingProposal, WordRules } from "@armada/protocol";
import type { TeachingApi } from "../shared/api/teaching";
import { ask, MODEL_CALL_MS } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

export class TeachingHost {
  private readonly port: () => number | null;

  constructor(port: () => number | null) {
    this.port = port;
  }

  async read(read: ReadTeaching): ReturnType<TeachingApi["readTeaching"]> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/teachings/read", read, MODEL_CALL_MS);
    return answer.ok === true ? { ok: true, value: answer.body as TeachingProposal } : { ok: false, outcome: answer.outcome };
  }

  async save(save: SaveTeaching): ReturnType<TeachingApi["saveTeaching"]> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const answer = await ask(port, "POST", "/teachings", save);
    return answer.ok === true ? { ok: true, value: answer.body as SavedTeaching } : { ok: false, outcome: answer.outcome };
  }

  /** `repository` and `sessionId` ride the query when named, and are encoded: a root is a path. */
  async rules(repository?: string, sessionId?: string): ReturnType<TeachingApi["listWordRules"]> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const query = new URLSearchParams();
    if (repository !== undefined) query.set("repository", repository);
    if (sessionId !== undefined) query.set("session_id", sessionId);
    const text = query.toString();
    const answer = await ask(port, "GET", text === "" ? "/teachings/words" : `/teachings/words?${text}`);
    return answer.ok === true ? { ok: true, value: answer.body as WordRules } : { ok: false, outcome: answer.outcome };
  }
}
