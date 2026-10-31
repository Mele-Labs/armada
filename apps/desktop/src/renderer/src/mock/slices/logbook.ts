// The Logbook: the scenario's entries, as edits, pins and forgets leave them for the next read. A forgotten entry is
// gone from every read, as the deny list keeps it out. Mock of Fleet's routes, which main reaches in `main/logbook.ts`.

import type { LogbookEntry, LogbookTidy } from "@armada/protocol";

import type { LogbookApi, OrganizeAction, OrganizeRow } from "../../../../shared/api/logbook";
import type { Slice } from "../fake-context";
import { alsoOnTimePassing } from "../time-passes";

/** How long a pass runs in the mock, as a real one runs in the background. A walk's `later` ends it sooner. */
const PASS_MS = 6_000;

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

/** How long Organize looks in the mock before it has something to show. */
const LOOK_MS = 1_500;
const STALE_DAYS = 45;
const FAULTS: [RegExp, string][] = [
  [/refus/i, "A tool refuses"],
  [/\berror|\bfails\b|\bfailed\b/i, "A tool errors"],
  [/work ?around|instead of|rejected|blocked/i, "A workaround"],
];

const entriesIn = (entries: LogbookEntry[], ids: readonly string[]) => entries.filter((one) => ids.includes(one.id));

/** What Organize finds in the mock, from the entries as they stand. */
function found(action: OrganizeAction, entries: LogbookEntry[]): OrganizeRow[] {
  const standing = entries.filter((one) => one.state === "standing" && !one.locked);
  if (action === "merge") {
    const done = tidied(entries);
    return done.entries
      .filter((one) => one.history.some((earlier) => earlier.by === "merged") && one.state === "standing")
      .map((one) => ({ id: one.id, text: one.title ?? one.text, note: "", absorbed: one.history.filter((earlier) => earlier.by === "merged").map((earlier) => earlier.text) }));
  }
  if (action === "stale") {
    const cutoff = Date.now() - STALE_DAYS * 86_400_000;
    return standing
      .filter((one) => Date.parse(one.last_confirmed) < cutoff)
      .map((one) => ({ id: one.id, text: one.title ?? one.text, note: `Last confirmed ${Math.round((Date.now() - Date.parse(one.last_confirmed)) / 86_400_000)} days ago` }));
  }
  return standing.flatMap((one) => {
    const fault = FAULTS.find(([words]) => words.test(one.text));
    return fault === undefined ? [] : [{ id: one.id, text: one.title ?? one.text, note: fault[1] }];
  });
}

export const logbook: Slice<LogbookApi, Record<string, never>> = {
  name: "logbook",
  state: {},
  api: (scenario, fleet) => {
    let held: LogbookEntry[] | undefined = scenario.logbook;
    let tidy: LogbookTidy = { running: false };
    let ending: ReturnType<typeof setTimeout> | undefined;
    const changed = (id: string, change: (entry: LogbookEntry) => LogbookEntry) => {
      const entry = held?.find((one) => one.id === id);
      if (held === undefined || entry === undefined) return fleet.refused(`/logbook/${id}`);
      const now = change(entry);
      held = held.map((one) => (one.id === id ? now : one));
      return { ok: true as const, entry: now };
    };
    return {
      readLogbook: async () => (held === undefined ? fleet.refused("/logbook") : { ok: true, entries: held, tidy }),
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
        if (tidy.running) return { ok: true, running: true };
        const finish = () => {
          clearTimeout(ending);
          if (!tidy.running || held === undefined) return;
          const done = tidied(held);
          held = done.entries;
          tidy = { running: false, last: { merged: done.merged, at: new Date().toISOString(), failed: false } };
        };
        tidy = { ...tidy, running: true };
        ending = setTimeout(finish, PASS_MS);
        alsoOnTimePassing(finish);
        return { ok: true, running: true };
      },
      organizeLogbook: async (action) => {
        await new Promise((done) => setTimeout(done, LOOK_MS));
        return held === undefined ? [] : found(action, held);
      },
      applyOrganizeLogbook: async (action, rows) => {
        if (held === undefined) return "";
        const ids = rows.map((row) => row.id);
        if (action === "merge") {
          const absorbed = new Map(rows.map((row) => [row.id, row.absorbed ?? []]));
          const at = new Date().toISOString();
          const gone = new Set(held.filter((one) => ids.some((id) => (absorbed.get(id) ?? []).includes(one.text))).map((one) => one.id));
          held = held.map((one) => {
            if (gone.has(one.id)) return { ...one, state: "faded" as const };
            if (!ids.includes(one.id)) return one;
            return { ...one, history: [...(absorbed.get(one.id) ?? []).map((text) => ({ text, until: at, by: "merged" as const })), ...one.history] };
          });
          return `Merged ${gone.size} ${gone.size === 1 ? "entry" : "entries"}`;
        }
        if (action === "stale") {
          held = held.map((one) => (ids.includes(one.id) ? { ...one, state: "faded" as const } : one));
          return `Archived ${ids.length} ${ids.length === 1 ? "entry" : "entries"}`;
        }
        void entriesIn;
        return "Reported to the Armada bug list";
      },
      forgetLogbookEntry: async (id) => {
        if (held === undefined || !held.some((one) => one.id === id)) return fleet.refused(`/logbook/${id}/forget`);
        held = held.filter((one) => one.id !== id);
        return { ok: true };
      },
    };
  },
};
