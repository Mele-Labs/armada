// The Logbook with near-duplicate commands, entries nothing has confirmed for months, and entries that read as workarounds for
// a fault in Armada itself, ready for Organize. The walk `logbook-organize` plays it.

import type { LogbookEntry } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { holding } from "../holding";
import type { Scenario } from "../moment";
import { logbookTidy } from "./logbook-tidy";

const ARMADA = repository().root;
const DAY = 86_400_000;
const ago = (days: number) => new Date(Date.now() - days * DAY).toISOString();

function entry(id: string, kind: LogbookEntry["kind"], text: string, confirmed_by: number, days: number): LogbookEntry {
  return { id, section: "repo", repository: ARMADA, kind, text, evidence: [], confirmed_by, first_seen: ago(days + 30), last_confirmed: ago(days), state: "standing", locked: false, edited: false, history: [] };
}

const MORE: LogbookEntry[] = [
  entry("old-port", "environment", "The mock server listens on port 5173. Nothing else uses it.", 2, 96),
  entry("old-lint", "convention", "Run `pnpm lint:fix` before every commit.", 1, 71),
  entry("old-yarn", "command", "Install with `yarn install --frozen-lockfile`.", 1, 130),
  entry("bug-start", "gotcha", "`start_server` refuses a job given as the slot path. Start vite directly from `apps/desktop` instead.", 4, 3),
  entry("bug-guard", "gotcha", "The slot guard errors on `git branch -D`. Work around it by leaving the branch for finish_up.", 3, 5),
  entry("bug-watch", "gotcha", "`try_watch` fails when the command is a shell line. Pass a list of arguments.", 2, 6),
];

export const logbookOrganize: Scenario = {
  ...holding("logbook-organize", "The Logbook with duplicates, stale entries and workarounds, ready for Organize", [repository()]),
  logbook: [...(logbookTidy.logbook ?? []), ...MORE],
};
