// The Logbook page's arithmetic: which entries a filter shows, grouped into the page's panels. `docs/concepts/logbook.md`.

import type { LogbookEntry, LogbookState } from "@armada/protocol";

/** One panel of the page: a repository's Logbook, the owner's habits, or this Mac. */
export type LogbookGroup = { key: string; title: string; hint: string; entries: LogbookEntry[] };

const lastPart = (root: string): string => root.split("/").filter(Boolean).pop() ?? root;

/** Tripped first, then locked, then the most recently confirmed. */
function sorted(list: LogbookEntry[]): LogbookEntry[] {
  const rank = (one: LogbookEntry): [number, number, number] => [
    one.last_tripped === undefined ? 1 : 0,
    one.pinned ? 0 : 1,
    -Date.parse(one.last_confirmed),
  ];
  return [...list].sort((a, b) => {
    const [x, y] = [rank(a), rank(b)];
    return x[0] - y[0] || x[1] - y[1] || x[2] - y[2];
  });
}

/**
 * The entries in one state, grouped: each repository in the order Fleet lists them, then Habits, then This Mac.
 */
export function groupsOf(entries: readonly LogbookEntry[], state: LogbookState): LogbookGroup[] {
  const shown = entries.filter((one) => one.state === state);
  const roots = [...new Set(shown.filter((one) => one.section === "repo").map((one) => one.repository ?? ""))];
  const groups: LogbookGroup[] = roots.map((root) => ({
    key: root,
    title: lastPart(root),
    hint: `Repository: ${lastPart(root)}`,
    entries: sorted(shown.filter((one) => one.section === "repo" && one.repository === root)),
  }));
  groups.push({ key: "habits", title: "Habits", hint: "How you work, handed out in every repository", entries: sorted(shown.filter((one) => one.section === "habits")) });
  groups.push({ key: "machine", title: "This Mac", hint: "What this Mac has and lacks", entries: sorted(shown.filter((one) => one.section === "machine")) });
  return groups.filter((group) => group.entries.length > 0);
}

/** Every entry in the order the page draws them, which is the order the arrows move in. */
export function orderOf(groups: readonly { entries: readonly LogbookEntry[] }[]): LogbookEntry[] {
  return groups.flatMap((group) => [...group.entries]);
}
