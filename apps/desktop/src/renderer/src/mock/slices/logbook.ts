// The Logbook: the scenario's entries, as edits, pins and forgets leave them for the next read. A forgotten entry is
// gone from every read, as the deny list keeps it out. Mock of Fleet's routes, which main reaches in `main/logbook.ts`.

import type { LogbookEntry, LogbookTidy, OrganizeAction, OrganizeProposal } from "@armada/protocol";

import type { LogbookApi } from "../../../../shared/api/logbook";
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

/** Which standing entries read alike: each folded entry's id against the survivor it folds into, the one confirmed most. */
function foldings(entries: LogbookEntry[]): Map<string, string> {
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
  return folded;
}

/** The folded go to Archived and the survivor holds their text under Earlier as Merged. */
function folding(entries: LogbookEntry[], folded: ReadonlyMap<string, string>): LogbookEntry[] {
  const at = new Date().toISOString();
  return entries.map((one) => {
    if (folded.has(one.id)) return { ...one, state: "faded" as const };
    const into = entries.filter((other) => folded.get(other.id) === one.id);
    if (into.length === 0) return one;
    return { ...one, history: [...into.map((other) => ({ text: other.text, until: at, by: "merged" as const })), ...one.history] };
  });
}

/** Tidy as the mock keeps it. */
function tidied(entries: LogbookEntry[]): { entries: LogbookEntry[]; merged: number } {
  const folded = foldings(entries);
  return { entries: folding(entries, folded), merged: folded.size };
}

/** How long Organize looks in the mock before it has something to show. */
const LOOK_MS = 1_500;
const STALE_DAYS = 45;
const FAULTS: [RegExp, string][] = [
  [/refus/i, "A tool refuses"],
  [/\berror|\bfails\b|\bfailed\b/i, "A tool errors"],
  [/work ?around|instead of|rejected|blocked/i, "A workaround"],
];

/** What Organize proposes in the mock, from the entries as they stand. */
function found(action: OrganizeAction, entries: LogbookEntry[]): OrganizeProposal[] {
  const standing = entries.filter((one) => one.state === "standing" && !one.locked);
  if (action === "merge") {
    const folded = foldings(entries);
    const kept = [...new Set(folded.values())];
    return kept.map((id) => ({
      id,
      entry_ids: [id, ...[...folded].filter(([, into]) => into === id).map(([gone]) => gone)],
      text: entries.find((one) => one.id === id)?.text ?? "",
      note: "",
    }));
  }
  if (action === "stale") {
    const cutoff = Date.now() - STALE_DAYS * 86_400_000;
    return standing
      .filter((one) => Date.parse(one.last_confirmed) < cutoff)
      .map((one) => ({ id: one.id, entry_ids: [one.id], text: one.text, note: `Not confirmed in ${Math.round((Date.now() - Date.parse(one.last_confirmed)) / 86_400_000)} days. Archived on apply` }));
  }
  return standing.flatMap((one) => {
    const fault = FAULTS.find(([words]) => words.test(one.text));
    return fault === undefined ? [] : [{ id: one.id, entry_ids: [one.id], text: one.text, note: `${fault[1]}. Reported on apply` }];
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
        if (held === undefined) return fleet.refused("/logbook/organize");
        await new Promise((done) => setTimeout(done, LOOK_MS));
        return { ok: true, action, proposals: found(action, held), failed: false };
      },
      applyOrganizeLogbook: async (action, proposals) => {
        if (held === undefined) return fleet.refused("/logbook/organize/apply");
        if (action === "merge") {
          const folded = new Map(proposals.flatMap((one) => one.entry_ids.slice(1).map((gone): [string, string] => [gone, one.entry_ids[0]!])));
          held = folding(held, folded);
          return { ok: true, action, applied: folded.size, reported: [] };
        }
        if (action === "stale") {
          const ids = proposals.flatMap((one) => one.entry_ids);
          held = held.map((one) => (ids.includes(one.id) ? { ...one, state: "faded" as const } : one));
          return { ok: true, action, applied: ids.length, reported: [] };
        }
        return { ok: true, action, applied: 0, reported: proposals };
      },
      forgetLogbookEntry: async (id) => {
        if (held === undefined || !held.some((one) => one.id === id)) return fleet.refused(`/logbook/${id}/forget`);
        held = held.filter((one) => one.id !== id);
        return { ok: true };
      },
    };
  },
};
