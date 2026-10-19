// A bay's side panel with every act the Worktree Slots panel offers and its history of leases. The
// cockpit ship's pool, plus slot-10: leased by an agent from a terminal (`armada worktree lease`), which Fleet's timeline never
// saw, so its history comes from `getSlotHistory`, which serves `draft.bayHistories`, and not from slot events. Held, free,
// closed and stranded bays each have a history, the held one a long one. The claims are in `walks/bay-pane-acts.ts`.

import type { BayHistory, BayHistoryEntry, BayHolder, WorktreeSlot } from "@armada/protocol";

import type { Scenario } from "../moment";
import { s252CockpitShipFlightdeck as ship } from "./cockpit-ship";

const HOUR = 60;
const ago = (minutes: number) => new Date(Date.now() - minutes * 60_000).toISOString();

const ARMADA = ship.held?.slots?.find((one) => one.slot === 2)?.manifest_id ?? "armada";
const POCKET = "pocket";

/** The Sessions the ship holds on a bay, so a history row that names one opens it. */
const sessionOn = (slot: number): BayHolder => {
  const held = ship.held?.slots?.find((one) => one.manifest_id === ARMADA && one.slot === slot)?.held;
  return held?.state === "hosted_session" ? { kind: "session", id: held.session_id, title: held.session_title } : { kind: "session", id: "gone" };
};
const PIN_CLOCK: BayHolder = { kind: "session", id: "r1", title: "Pin the store clock" };
const GATE_POLICY: BayHolder = { kind: "session", id: "p1", title: "Gate policy on every run" };
const CHECKS: BayHolder = { kind: "agent", id: "claude (pid 44698)", title: "Checks --changed Bug" };
const CHECKS_EARLIER: BayHolder = { kind: "agent", id: "claude (pid 31207)", title: "Checks --changed Store" };
const FRAME: BayHolder = { kind: "session", id: "01SESSIONCLOSEDFRAME0000", title: "Frame the reply" };

const leased = (minutes: number, holder: BayHolder, branch?: string): BayHistoryEntry => ({ kind: "leased", at: ago(minutes), holder, ...(branch === undefined ? {} : { branch }) });
const released = (minutes: number, holder: BayHolder): BayHistoryEntry => ({ kind: "released", at: ago(minutes), holder });
const checkout = (minutes: number, sha: string, to: string, from?: string): BayHistoryEntry => ({ kind: "checkout", at: ago(minutes), sha, to, ...(from === undefined ? {} : { from }) });

const history = (manifest: string, slot: number, entries: BayHistoryEntry[], cut = false): BayHistory => ({
  manifest_id: manifest,
  slot,
  entries: [...entries].sort((a, b) => Date.parse(b.at) - Date.parse(a.at)),
  cut,
});

const HISTORIES: BayHistory[] = [
  // Held by a terminal's agent for 65 hours, on one branch and then others, after a run of holders the timeline never named.
  history(
    ARMADA,
    10,
    [
      leased(65 * HOUR, CHECKS, "armada/session-627d8d8f"),
      checkout(65 * HOUR - 2, "4be09d1", "armada/session-627d8d8f", "main"),
      checkout(52 * HOUR, "9a1c77e", "fix/bay-history", "armada/session-627d8d8f"),
      checkout(51 * HOUR + 40, "9a1c77e", "armada/session-627d8d8f", "fix/bay-history"),
      checkout(30 * HOUR, "c20f5b8", "main", "armada/session-627d8d8f"),
      checkout(29 * HOUR + 55, "71d3a04", "armada/session-627d8d8f", "main"),
      checkout(5, "f19a2c6", "armada/session-627d8d8f", "fix/checks-changed"),
      checkout(40, "f19a2c6", "fix/checks-changed", "armada/session-627d8d8f"),
      checkout(7 * HOUR, "e5b8130", "fix/checks-changed", "armada/session-627d8d8f"),
      checkout(6 * HOUR + 12, "e5b8130", "armada/session-627d8d8f", "fix/checks-changed"),
      released(66 * HOUR + 20, CHECKS_EARLIER),
      leased(70 * HOUR + 5, CHECKS_EARLIER, "armada/session-0e41b6c2"),
      checkout(69 * HOUR, "b7d2ee0", "main", "armada/session-0e41b6c2"),
      released(72 * HOUR + 30, sessionOn(2)),
      leased(81 * HOUR, sessionOn(2), "fix/cache-key"),
      checkout(80 * HOUR + 50, "3fa9c14", "fix/cache-key"),
      released(90 * HOUR, PIN_CLOCK),
      leased(97 * HOUR, PIN_CLOCK, "fix/pin-store-clock"),
      checkout(96 * HOUR + 30, "d18e6a5", "fix/pin-store-clock", "main"),
    ],
    true,
  ),
  // Free: given back an hour ago, after two Sessions.
  history(ARMADA, 5, [
    leased(5 * HOUR, GATE_POLICY, "fleet/gate-policy-every-run"),
    checkout(4 * HOUR + 50, "a40c9e3", "fleet/gate-policy-every-run", "main"),
    released(3 * HOUR, GATE_POLICY),
    leased(2 * HOUR + 30, sessionOn(3), "fleet/reconnect-wait"),
    released(HOUR, sessionOn(3)),
  ]),
  // Stranded: its last holder is gone.
  history(ARMADA, 6, [
    leased(4 * HOUR, FRAME, "session/frame-the-reply"),
    checkout(3 * HOUR + 40, "e08c4d1", "session/frame-the-reply", "main"),
    released(3 * HOUR, FRAME),
  ]),
  // Closed: a person closed it after its last lease.
  history(POCKET, 2, [leased(30 * HOUR, { kind: "session", id: "01POCKETSESSIONAUDIT0000", title: "Audit the pairing flow" }, "pocket/audit"), checkout(29 * HOUR, "6c4d2f9", "pocket/audit", "main"), released(26 * HOUR, { kind: "session", id: "01POCKETSESSIONAUDIT0000", title: "Audit the pairing flow" })]),
];

function build(): Scenario {
  const cli: WorktreeSlot = {
    manifest_id: ARMADA,
    slot: 10,
    path: "/Users/user/armada/.armada/slots/slot-10",
    base: "main",
    warm: true,
    behind: 838,
    held: { state: "session", holder: "claude (pid 44698)", session_title: "Checks --changed Bug" },
    branch: "armada/session-627d8d8f",
    since: ago(65 * HOUR),
  };
  const slots = [...(ship.held?.slots ?? []), cli];
  return {
    ...ship,
    name: "bay-pane-acts",
    says: "The flight deck's bay pane: every act of the Worktree Slots panel on a held, free, closed and stranded bay, and each bay's history of leases and checkouts",
    held: { slots },
    draft: { ...ship.draft, bayHistories: HISTORIES },
  };
}

export const s260BayPaneActs: Scenario = build();
