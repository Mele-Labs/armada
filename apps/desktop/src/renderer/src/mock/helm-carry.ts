// A mock Fleet that serves Helm as the Session it is: Sessions answered by `FakeSessionsFleet`, and the
// four things only Helm's dock asks — a message, a thought recorded, Start fresh, and the reading of a
// thought. The first message of a repository (or the first thought recorded against it) makes its Helm
// Session, and what follows is rows in that Session's thread, which the window reads as it reads any
// Session's. Outside `scenarios/`, where every export is read as a row.

import type { SessionRecord, SessionRow, SessionTag, SessionUpload, ThoughtTarget } from "@armada/protocol";

import type { BridgeApi } from "../../../shared/api";
import type { HelmCarry } from "../../../shared/api/helm";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import type { FleetHandle, Scenario } from "./moment";
import { readAfterPause } from "./thought-reading";

/** What `/` offers in a message box: the agent's own skills and commands, as its init line names them. */
export const COMMANDS = ["review", "simplify", "security-review", "init", "compact", "clear", "model"];

const now = () => new Date().toISOString();

/** What an upload or a tag becomes on a row of the thread, the way Fleet keeps it. */
function carried(rowId: string, carry: { attachments?: SessionUpload[]; mentions?: SessionTag[] } | undefined) {
  const files = (carry?.attachments ?? []).map((upload, at) => ({ id: `${rowId}-${at}`, name: upload.name, media_type: upload.media_type }));
  const tags = carry?.mentions ?? [];
  return { ...(files.length === 0 ? {} : { files }), ...(tags.length === 0 ? {} : { tags }) };
}

/**
 * `scenario`'s Sessions and Jobs, served by a Fleet of this mount's own, with Helm's acts on top. `reply` is
 * what Helm says to any message. The Helm Session of a repository is made by its first message.
 */
export function helmFleetOn(scenario: Scenario, handle: FleetHandle, reply: string): Partial<BridgeApi> {
  const fleet = new FakeSessionsFleet(scenario.state.sessions.state === "read" ? [...scenario.state.sessions.sessions] : [], {});
  // The dock opens the thread after the first message makes the Session, so what was said before is kept for it.
  fleet.keepsRows = true;
  const sessions = fleet.scenario(scenario).behaves!(handle);
  const live = new Map<string, SessionRecord>();
  let rowed = 0;
  const id = () => `helm-row-${(rowed += 1)}`;
  /** The repository the dock answers for. */
  const aimed = (): string => {
    const aim = handle.state().helm;
    return aim.state === "pointed" ? aim.manifestId : "armada";
  };
  /** Helm's Session for a repository, made now if it has none. */
  const session = (manifestId: string, tuning: { model?: string; effort?: string } = {}): SessionRecord => {
    const found = live.get(manifestId);
    if (found !== undefined) return found;
    const made = hosted(`01HELM${manifestId.toUpperCase().padEnd(20, "A")}`.slice(0, 26), {
      helm: true,
      manifest_id: manifestId,
      title: `Helm · ${manifestId}`,
      last_turn_at: now(),
      hosted: { turn: { state: "idle" }, mode: "auto", running: true, commands: COMMANDS, ...tuning },
    });
    live.set(manifestId, made);
    fleet.changed(made);
    return made;
  };
  const row = (sessionId: string, make: SessionRow): void => fleet.row(sessionId, make);
  const working = (record: SessionRecord, turn: "working" | "idle"): void => {
    const next = { ...record, hosted: { ...record.hosted!, turn: { state: turn } }, ...(turn === "idle" ? { last_turn_at: now() } : {}) } as SessionRecord;
    live.set(record.manifest_id!, next);
    fleet.changed(next);
  };

  return {
    ...sessions,
    // Fleet's reading of a thought: the mock's keyword scorer over this scenario's own Sessions. Helm's own
    // Session is not a place a thought goes.
    readThought: (text) => {
      const state = handle.state();
      return readAfterPause(text, {
        sessions: state.sessions.state === "read" ? state.sessions.sessions.filter((one) => one.helm !== true) : [],
      });
    },
    askHelm: async (text, _context, carry?: HelmCarry) => {
      const record = session(aimed(), { ...(carry?.model === undefined ? {} : { model: carry.model }), ...(carry?.effort === undefined ? {} : { effort: carry.effort }) });
      const at = now();
      const said = id();
      row(record.id, { kind: "message", id: said, at, from: { kind: "you" }, text, ...carried(said, carry) });
      working(record, "working");
      // Helm's reply is Fleet's to write; the mock gives the one answer the scenario asks for.
      row(record.id, { kind: "message", id: id(), at, from: { kind: "agent" }, text: reply });
      working(live.get(record.manifest_id!)!, "idle");
      return { ok: true };
    },
    // Fleet keeps the row for words sent on, after the fact, in the Session of the repository that was asked about.
    recordThoughtSent: async (manifestId, text: string, target: ThoughtTarget, carry?: HelmCarry) => {
      const record = session(manifestId);
      const sent = id();
      row(record.id, { kind: "sent", id: sent, at: now(), text, target, ...carried(sent, carry) });
      return { ok: true };
    },
    // The Session ends and the next message starts a new one.
    startHelmFresh: async () => {
      const record = live.get(aimed());
      if (record === undefined) return { ok: true };
      live.delete(aimed());
      fleet.changed({ ...record, state: "ended" });
      return { ok: true };
    },
  };
}
