// Autopilot's members: one repository's Autopilot mid-run, so the Cockpit lane has something to draw.
// Two picks started, one set aside with its reason, and one withdrawn by an earlier Stop. The acts
// move it as Fleet would; what Fleet keeps and picks is `fleet::autopilot`'s. #2092.

import type { Autopilot, AutopilotProposal } from "@armada/protocol";

import type { AutopilotApi } from "../../../../shared/api/autopilot";
import type { Slice } from "../fake-context";

function running(manifestId: string): Autopilot {
  return {
    manifest_id: manifestId,
    instruction: "work through the open issues that don't need design decisions",
    at_once: 2,
    until: "2026-10-12T11:00:00.000Z",
    spend_cap_micros: 30_000_000,
    state: "running",
    started_at: "2026-10-12T03:00:00.000Z",
    spent_micros: 4_120_000,
    jobs_started: 2,
    at_work: 1,
    picks: [
      {
        pick_id: "01JAUTOPILOTPICK0000000001",
        issue_number: 412,
        issue_title: "The log reader drops the last line",
        issue_url: "https://forge.invalid/storefront/issues/412",
        workflow_id: "bug",
        job_id: "01JAUTOPILOTJOB00000000001",
        state: "started",
        reason: "A reader bug with a failing case in the issue; one file.",
        at: "2026-10-12T03:00:40.000Z",
      },
      {
        pick_id: "01JAUTOPILOTPICK0000000002",
        issue_number: 377,
        issue_title: "Should the Board group Jobs by repository?",
        issue_url: "https://forge.invalid/storefront/issues/377",
        workflow_id: "feature",
        state: "set_aside",
        reason: "It asks how the Board should look, which is a design decision.",
        at: "2026-10-12T03:01:10.000Z",
      },
      {
        pick_id: "01JAUTOPILOTPICK0000000003",
        issue_number: 401,
        issue_title: "Checkout total rounds the tax twice",
        issue_url: "https://forge.invalid/storefront/issues/401",
        workflow_id: "bug",
        job_id: "01JAUTOPILOTJOB00000000002",
        state: "started",
        reason: "A rounding bug the issue reproduces; the fix is in one function.",
        at: "2026-10-12T03:01:30.000Z",
      },
    ],
  };
}

export const autopilot: Slice<AutopilotApi, Record<string, never>> = {
  name: "autopilot",
  state: {},
  api: () => {
    const held = new Map<string, Autopilot>();
    const told = new Set<(autopilot: Autopilot) => void>();
    const of = (manifestId: string): Autopilot => held.get(manifestId) ?? running(manifestId);
    const moved = (manifestId: string, change: Partial<Autopilot>) => {
      const now = { ...of(manifestId), ...change };
      held.set(manifestId, now);
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
      startAutopilot: async (manifestId, settings) =>
        moved(manifestId, { ...settings, state: "running", picks: [], spent_micros: 0, jobs_started: 0, at_work: 0, ended_at: undefined, capped_by: undefined }),
      updateAutopilot: async (manifestId, settings) => moved(manifestId, { until: undefined, spend_cap_micros: undefined, job_cap: undefined, ...settings }),
      pauseAutopilot: async (manifestId) => moved(manifestId, { state: "paused" }),
      resumeAutopilot: async (manifestId) => moved(manifestId, { state: "running", capped_by: undefined }),
      stopAutopilot: async (manifestId) => moved(manifestId, { state: "stopped", ended_at: "2026-10-12T07:12:00.000Z" }),
      listAutopilots: async () => ({ ok: true, value: { autopilots: [...held.values()] } }),
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
