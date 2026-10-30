// The bug report routes, answered from a scenario. A scenario hands the bugs agents reported (`MockDraft.reportedBugs`), and
// Keep and Drop below change this window's copy, answer as Fleet does, and then publish the whole list as `bug_reports.changed`.

import type { BugReport, BugReports } from "@armada/protocol";

import type { BugReportsApi } from "../../../../shared/api/bug-reports";
import type { Slice } from "../fake-context";

const noReport = { ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.no_such_bug_report", message: "There is no such bug report", run_id: "mock", fields: {}, chain: [] } } as never } as const;

export const bugReports: Slice<BugReportsApi, Record<string, never>> = {
  name: "bugReports",
  state: {},
  api: (scenario) => {
    // The seed's `kept` is a flag; Fleet's is the time it was kept, which a seeded one gets as the time it was reported.
    let held: BugReport[] = (scenario.draft?.reportedBugs ?? []).map(({ kept, ...one }) => ({ ...one, steps: [...one.steps], errors: [...one.errors], logs: [...one.logs], ...(kept ? { kept: one.at } : {}) }));
    const listeners = new Set<(reports: BugReports) => void>();
    const listed = (): BugReports => ({ reports: structuredClone(held) });
    /** A write: answers, and a moment after publishes the list whole, as Fleet's event follows its reply. */
    const changed = () => setTimeout(() => listeners.forEach((on) => on(listed())), 0);
    return {
      listBugReports: async () => ({ ok: true, value: listed() }),
      keepBugReport: async (id) => {
        const one = held.find((it) => it.id === id);
        if (one === undefined) return noReport;
        held = held.map((it) => (it.id === id ? { ...it, kept: it.kept ?? new Date().toISOString() } : it));
        changed();
        return { ok: true, value: structuredClone(held.find((it) => it.id === id)!) };
      },
      dropBugReport: async (id) => {
        if (!held.some((it) => it.id === id)) return noReport;
        held = held.filter((it) => it.id !== id);
        changed();
        return { ok: true, value: { id } };
      },
      onBugReportsChanged: (onChanged) => {
        listeners.add(onChanged);
        return () => void listeners.delete(onChanged);
      },
    };
  },
};
