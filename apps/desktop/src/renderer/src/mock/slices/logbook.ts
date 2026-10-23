// The Logbook: the scenario's entries, as edits, pins and forgets leave them for the next read. A forgotten entry is
// gone from every read, as the deny list keeps it out. Mock of Fleet's routes, which main reaches in `main/logbook.ts`.

import type { LogbookEntry } from "@armada/protocol";

import type { LogbookApi } from "../../../../shared/api/logbook";
import type { Slice } from "../fake-context";

const words = (text: string): Set<string> => new Set(text.toLowerCase().match(/[a-z0-9]+/g) ?? []);
const alike = (a: string, b: string): boolean => {
  const [x, y] = [words(a), words(b)];
  const shared = [...x].filter((word) => y.has(word)).length;
  return shared / new Set([...x, ...y]).size >= 0.6;
};

/**
 * Tidy as the mock keeps it: standing entries of one section, repository and kind that read alike fold into the one
 * confirmed most. The folded go to Archived and the survivor holds their text under Earlier as Merged.
 */
function tidied(entries: LogbookEntry[]): { entries: LogbookEntry[]; merged: number } {
  const at = new Date().toISOString();
  const folded = new Map<string, string>();
  const standing = entries.filter((one) => one.state === "standing" && !one.locked).sort((a, b) => b.confirmed_by - a.confirmed_by);
  for (const survivor of standing) {
    if (folded.has(survivor.id)) continue;
    for (const other of standing) {
      if (other.id === survivor.id || folded.has(other.id)) continue;
      const same = other.section === survivor.section && other.repository === survivor.repository && other.kind === survivor.kind;
      if (same && alike(survivor.text, other.text)) folded.set(other.id, survivor.id);
    }
  }
  const next = entries.map((one) => {
    if (folded.has(one.id)) return { ...one, state: "faded" as const };
    const into = entries.filter((other) => folded.get(other.id) === one.id);
    if (into.length === 0) return one;
    return { ...one, history: [...into.map((other) => ({ text: other.text, until: at, by: "merged" as const })), ...one.history] };
  });
  return { entries: next, merged: folded.size };
}

export const logbook: Slice<LogbookApi, Record<string, never>> = {
  name: "logbook",
  state: {},
  api: (scenario, fleet) => {
    let held: LogbookEntry[] | undefined = scenario.logbook;
    const changed = (id: string, change: (entry: LogbookEntry) => LogbookEntry) => {
      const entry = held?.find((one) => one.id === id);
      if (held === undefined || entry === undefined) return fleet.refused(`/logbook/${id}`);
      const now = change(entry);
      held = held.map((one) => (one.id === id ? now : one));
      return { ok: true as const, entry: now };
    };
    return {
      readLogbook: async () => (held === undefined ? fleet.refused("/logbook") : { ok: true, entries: held }),
      editLogbookEntry: async (id, text) =>
        changed(id, (entry) => ({
          ...entry,
          text,
          edited: true,
          history: [{ text: entry.text, until: new Date().toISOString(), by: "person" as const }, ...entry.history],
        })),
      lockLogbookEntry: async (id, locked) => changed(id, (entry) => ({ ...entry, locked })),
      tidyLogbook: async () => {
        if (held === undefined) return fleet.refused("/logbook/tidy");
        const done = tidied(held);
        held = done.entries;
        return { ok: true, merged: done.merged };
      },
      forgetLogbookEntry: async (id) => {
        if (held === undefined || !held.some((one) => one.id === id)) return fleet.refused(`/logbook/${id}/forget`);
        held = held.filter((one) => one.id !== id);
        return { ok: true };
      },
    };
  },
};
