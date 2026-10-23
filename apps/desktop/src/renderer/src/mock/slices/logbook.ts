// The Logbook: the scenario's entries, as edits, pins and forgets leave them for the next read. A forgotten entry is
// gone from every read, as the deny list keeps it out. Mock only: no Fleet route serves the Logbook yet.

import type { LogbookEntry } from "@armada/protocol";

import type { LogbookApi } from "../../../../shared/api/logbook";
import type { Slice } from "../fake-context";

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
      pinLogbookEntry: async (id, pinned) => changed(id, (entry) => ({ ...entry, pinned })),
      forgetLogbookEntry: async (id) => {
        if (held === undefined || !held.some((one) => one.id === id)) return fleet.refused(`/logbook/${id}/forget`);
        held = held.filter((one) => one.id !== id);
        return { ok: true };
      },
    };
  },
};
