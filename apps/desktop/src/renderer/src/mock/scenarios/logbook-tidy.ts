// A repository's Logbook with the same two commands taught many times over, in slightly different words, all in use.
// Tidy now folds each run into the entry confirmed most. The walk `logbook-tidy` plays it.

import type { LogbookEntry } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";

const ARMADA = repository().root;
const DAY = 86_400_000;
const ago = (days: number) => new Date(Date.now() - days * DAY).toISOString();

function command(id: string, text: string, confirmed_by: number, days: number): LogbookEntry {
  return { id, section: "repo", repository: ARMADA, kind: "command", text, evidence: [], confirmed_by, first_seen: ago(days + 10), last_confirmed: ago(days), state: "standing", locked: false, edited: false, history: [] };
}

const ENTRIES: LogbookEntry[] = [
  command("vitest-1", "Run `pnpm exec vitest run <touched files>`. Never the whole desktop suite.", 7, 1),
  command("vitest-2", "Run `pnpm exec vitest run <touched files>` only. Never the whole desktop suite.", 3, 2),
  command("vitest-3", "Use `pnpm exec vitest run <touched files>`. Never the whole desktop suite.", 2, 3),
  command("vitest-4", "Run `pnpm exec vitest run <touched files>`, not the whole desktop suite.", 2, 4),
  command("vitest-5", "Run `pnpm exec vitest run <files you touched>`. Never the whole desktop suite.", 2, 5),
  command("vitest-6", "Run `pnpm exec vitest run <touched files>`. Do not run the whole desktop suite.", 2, 6),
  command("land-1", "Land with `scripts/land`. The merge line takes no new branches.", 5, 7),
  command("land-2", "Land with `scripts/land`, since the merge line takes no new branches.", 3, 8),
  command("land-3", "Land through `scripts/land`. The merge line takes no new branches.", 2, 9),
  command("land-4", "Land with `scripts/land`. The merge line takes no new branches now.", 2, 10),
  command("migrate", "Run `make migrate` before the tests. A fresh checkout has no schema.", 3, 11),
];

export const logbookTidy: Scenario = {
  ...holding("logbook-tidy", "The Logbook with two commands taught many times in near-identical words, ready for Tidy now", [repository()]),
  logbook: ENTRIES,
};
