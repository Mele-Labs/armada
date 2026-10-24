// The whole ship of `cockpit-ship`, and Fleet restarting under it. The first `later` drops the connection, and from there the scenario
// plays a restart on its own clock: unreachable, not running, connecting, starting for a few seconds, then connected with the merge
// line and the bays arriving after it, so the Cockpit's power-on and the battlefield's fill can be watched. The second `later` adds a
// bay in a third repository, whose territory grows in while the others glide. The walk `cockpit-power-on` plays it.

import type { Connection, FleetIdentity, MergeLines, WorktreesHeld } from "@armada/protocol";
import { connectedTo } from "@armada/protocol";

import type { BridgeState } from "../../../../shared/bridge";
import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const before = ship.state.connection;
const was: FleetIdentity = before.state === "connected" ? before.fleet : { protocolId: "0000000000000000", pid: 4242, port: 7878, startedAt: new Date().toISOString() };
const UNREACHABLE: Connection = { state: "unreachable", fleet: was, detail: "connection refused", sinceMs: 0 };

/** Each moment of the restart after the drop, and when it is published. */
function restart(lines: MergeLines | null, pool: WorktreesHeld | undefined): readonly { at: number; change: Partial<BridgeState> }[] {
  const now: FleetIdentity = { ...was, pid: was.pid + 1, startedAt: new Date().toISOString() };
  return [
    { at: 1_500, change: { connection: { state: "not_running", absence: { why: "pid_dead", path: "/Users/user/Library/Application Support/Armada/fleet.json", pid: was.pid } } } },
    { at: 3_000, change: { connection: { state: "connecting", fleet: now } } },
    { at: 3_600, change: { connection: { state: "starting", fleet: now } } },
    { at: 7_000, change: { connection: connectedTo(now, 1) } },
    { at: 7_400, change: { mergeLines: lines } },
    ...(pool === undefined ? [] : [{ at: 7_900, change: { held: { state: "read" as const, held: pool } } }]),
  ];
}

/** A free bay in a repository the field has not drawn, so a third territory comes in. */
function withThird(pool: WorktreesHeld | undefined): WorktreesHeld {
  const slots = pool?.slots ?? [];
  return { ...pool, slots: [...slots, { manifest_id: "tools", slot: 1, path: "/Users/user/tools/.armada/slots/slot-1", base: "main", warm: true, behind: 0, held: { state: "free" } }] };
}

export const s253FleetRestart: Scenario = {
  ...ship,
  name: "fleet-restart",
  says: "The whole ship, and Fleet restarting under it: the Cockpit stands by, then fills as Fleet answers",
  later: [{ connection: UNREACHABLE, mergeLines: null, held: { state: "reading" } }, { held: { state: "read", held: withThird(ship.held) } }],
  behaves: (fleet) => {
    // The drop is the first `later`; from it the restart plays on its own clock, once.
    const lines = fleet.state().mergeLines;
    let playing = false;
    window.setInterval(() => {
      if (playing || fleet.state().connection.state !== "unreachable") return;
      playing = true;
      for (const { at, change } of restart(lines, ship.held)) window.setTimeout(() => fleet.publish(change), at);
    }, 100);
    return ship.behaves?.(fleet) ?? {};
  },
};
