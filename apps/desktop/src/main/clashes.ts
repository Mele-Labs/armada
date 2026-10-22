// The write clashes standing in every repository Fleet serves, read per Manifest: `GET /clashes`
// names one repository, and bare it answers only the first served. Nothing is kept here, since
// Fleet's table is in memory and a read is cheap; the renderer decides when to ask again.

import type { ClashFile, ClashSide, Outcome, SessionFileChange, WriteClash, WriteClashes } from "@armada/protocol";
import type { ClashFileAnswer, SessionFileChangeAnswer, WriteClashesRead } from "../shared/api/clashes";
import { Picked } from "./picked";
import { ask, type Answer } from "./request";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };
const NAMING = new Picked();

/** A Fleet before 23.81 has no such route, and the router answers it 404 without a refusal's body. */
const hasNoRoute = (answer: Answer): boolean =>
  !answer.ok && answer.outcome.ok === false && answer.outcome.why === "transport" && answer.outcome.fault.why === "unanswerable" && answer.outcome.fault.status === 404;

export class ClashesHost {
  private readonly port: () => number | null;
  private readonly manifests: () => readonly string[] | null;

  /** `manifests` is the id of each served repository that has one; `null` before Fleet has listed any. */
  constructor(port: () => number | null, manifests: () => readonly string[] | null) {
    this.port = port;
    this.manifests = manifests;
  }

  async get(): Promise<WriteClashesRead> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const listed = this.manifests();
    const paths = listed === null ? ["/clashes"] : listed.map((id) => NAMING.manifestNamed("/clashes", id));
    const answers = await Promise.all(paths.map((path) => ask(port, "GET", path)));
    const clashes: WriteClash[] = [];
    for (const answer of answers) {
      if (hasNoRoute(answer)) continue;
      if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
      clashes.push(...(answer.body as WriteClashes).clashes);
    }
    return { ok: true, value: clashes };
  }

  /** One file of a clash. **The path crosses encoded**: a name can hold `&`, `#` or a space. */
  async file(sessionId: string, other: { kind: ClashSide; id: string }, path: string): Promise<ClashFileAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const query = new URLSearchParams({ session_id: sessionId, other_kind: other.kind, other_id: other.id, path });
    const answer = await ask(port, "GET", `/clashes/file?${query.toString()}`);
    return answer.ok === true ? { ok: true, value: answer.body as ClashFile } : { ok: false, outcome: answer.outcome };
  }

  /** One file a Job wrote, as its change since the base. `seen` is the stamp of the change already held. */
  async fileChange(sessionId: string, path: string, seen?: string): Promise<SessionFileChangeAnswer> {
    const port = this.port();
    if (port === null) return { ok: false, outcome: NOT_CONNECTED };
    const query = new URLSearchParams({ session_id: sessionId, path });
    if (seen !== undefined) query.set("seen", seen);
    const answer = await ask(port, "GET", `/sessions/file_change?${query.toString()}`);
    return answer.ok === true ? { ok: true, value: answer.body as SessionFileChange } : { ok: false, outcome: answer.outcome };
  }
}
