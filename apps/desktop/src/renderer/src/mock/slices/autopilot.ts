// Autopilot's members: one repository's Autopilot mid-run, so the Cockpit lane has something to draw.
// Two picks started, one set aside with its reason, and one withdrawn by an earlier Stop. `pocket`'s is
// paused by its Session cap, as is the one `autopilot-why` opens on. A start over a running or paused one
// is refused as Fleet refuses it, so it has to be stopped first. The acts move it as Fleet would; what
// Fleet keeps and picks is `fleet::autopilot`'s. #2092, #2105.

import type { Autopilot, AutopilotProposal, Outcome } from "@armada/protocol";

import type { AutopilotApi } from "../../../../shared/api/autopilot";
import type { Slice } from "../fake-context";
import { BACK_MANIFEST } from "./autopilot-decisions";

function running(manifestId: string): Autopilot {
  return {
    manifest_id: manifestId,
    instruction: "work through the open issues that don't need design decisions",
    at_once: 2,
    until: "2026-10-12T11:00:00.000Z",
    spend_cap_micros: 30_000_000,
    state: "running",
    started_at: "2026-10-12T03:00:00.000Z",
    on_autopilot: { backlog: true, running: false },
    decisions: [],
    spent_micros: 4_120_000,
    sessions_started: 2,
    at_work: 1,
    picks: [
      {
        pick_id: "01JAUTOPILOTPICK0000000001",
        issue_reference: `#${412}`,
        issue_title: "The log reader drops the last line",
        issue_url: "https://forge.invalid/storefront/issues/412",
        workflow_id: "bug",
        session_id: "01JAUTOPILOTSESSION000000001",
        state: "started",
        reason: "A reader bug with a failing case in the issue; one file.",
        at: "2026-10-12T03:00:40.000Z",
      },
      {
        pick_id: "01JAUTOPILOTPICK0000000002",
        issue_reference: `#${377}`,
        issue_title: "Should the Board group Sessions by repository?",
        issue_url: "https://forge.invalid/storefront/issues/377",
        workflow_id: "feature",
        state: "set_aside",
        reason: "It asks how the Board should look, which is a design decision.",
        at: "2026-10-12T03:01:10.000Z",
      },
      {
        pick_id: "01JAUTOPILOTPICK0000000003",
        issue_reference: `#${401}`,
        issue_title: "Checkout total rounds the tax twice",
        issue_url: "https://forge.invalid/storefront/issues/401",
        workflow_id: "bug",
        session_id: "01JAUTOPILOTSESSION000000002",
        state: "started",
        reason: "A rounding bug the issue reproduces; the fix is in one function.",
        at: "2026-10-12T03:01:30.000Z",
      },
    ],
  };
}

/** The second repository's: three Sessions started of a cap of three, so the cap paused it. */
function capped(manifestId: string): Autopilot {
  const started = (n: number, title: string): Autopilot["picks"][number] => ({
    pick_id: `01JAUTOPILOTPOCKETPICK00${n}`,
    issue_reference: `#${30 + n}`,
    issue_title: title,
    issue_url: `https://forge.invalid/pocket/issues/${30 + n}`,
    workflow_id: "bug",
    session_id: `01JAUTOPILOTPOCKETSESSION${n}`,
    state: "started",
    reason: "A small fix the issue reproduces.",
    at: `2026-10-12T03:0${n}:00.000Z`,
  });
  return {
    manifest_id: manifestId,
    instruction: "fix the pairing bugs, three Sessions at most",
    at_once: 1,
    session_cap: 3,
    state: "paused",
    capped_by: "sessions",
    started_at: "2026-10-12T03:00:00.000Z",
    on_autopilot: { backlog: true, running: false },
    decisions: [],
    spent_micros: 6_300_000,
    sessions_started: 3,
    at_work: 0,
    picks: [started(1, "The pairing code expires early"), started(2, "Pairing fails offline"), started(3, "The QR code is cut off")],
  };
}

const refused = (code: string, message: string): { ok: false; outcome: Outcome } => ({
  ok: false,
  outcome: { ok: false, why: "refused", error: { code, message, run_id: "", fields: {}, chain: [] } },
});

export const autopilot: Slice<AutopilotApi, Record<string, never>> = {
  name: "autopilot",
  state: {},
  api: (scenario) => {
    // `autopilot-back` is the Cockpit with Autopilot already running, started before the person left.
    // `autopilot-why` is the Cockpit after a cap brought it down while the person was away.
    const kept = new Map<string, Autopilot>(
      scenario.name === "autopilot-back"
        ? [[BACK_MANIFEST, { ...running(BACK_MANIFEST), decisions: ["recommended", "open", "land"], picks: [] }]]
        : scenario.name === "autopilot-why"
          ? [[BACK_MANIFEST, capped(BACK_MANIFEST)]]
          : [],
    );
    const told = new Set<(autopilot: Autopilot) => void>();
    const of = (manifestId: string): Autopilot => kept.get(manifestId) ?? (manifestId === "pocket" ? capped(manifestId) : running(manifestId));
    const moved = (manifestId: string, change: Partial<Autopilot>) => {
      const now = { ...of(manifestId), ...change };
      kept.set(manifestId, now);
      told.forEach((one) => one(now));
      return { ok: true as const, value: now };
    };
    const picked = (manifestId: string, pickId: string, state: "started" | "dismissed") =>
      moved(manifestId, {
        picks: of(manifestId).picks.map((pick) => (pick.pick_id === pickId ? { ...pick, state } : pick)),
      });
    return {
      proposeAutopilot: async (_manifestId: string, sentence: string) => {
        const proposal: AutopilotProposal = {
          instruction: sentence,
          at_once: 2,
          until: "2026-10-12T11:00:00.000Z",
          spend_cap_micros: 30_000_000,
          quoted: { instruction: sentence, at_once: "two at a time", until: "until 7am", spend_cap_micros: "$30 max" },
        };
        return { ok: true, value: proposal };
      },
      startAutopilot: async (manifestId, settings) => {
        const was = kept.get(manifestId)?.state;
        if (was === "running") return refused("fleet.autopilot_running", "Autopilot is running");
        if (was === "paused") return refused("fleet.autopilot_not_running", "Autopilot is paused");
        return moved(manifestId, { ...settings, state: "running", picks: [], spent_micros: 0, sessions_started: 0, at_work: 0, ended_at: undefined, capped_by: undefined });
      },
      stopAutopilot: async (manifestId) => moved(manifestId, { state: "stopped", ended_at: "2026-10-12T07:12:00.000Z" }),
      listAutopilots: async () => ({ ok: true, value: { autopilots: [...kept.values()] } }),
      getAutopilot: async (manifestId) => ({ ok: true, value: of(manifestId) }),
      dispatchSetAside: async (manifestId, pickId) => picked(manifestId, pickId, "started"),
      dismissSetAside: async (manifestId, pickId) => picked(manifestId, pickId, "dismissed"),
      onAutopilotChanged: (onChanged) => {
        told.add(onChanged);
        return () => void told.delete(onChanged);
      },
    };
  },
};
