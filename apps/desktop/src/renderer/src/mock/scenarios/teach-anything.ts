// Teaching Armada from anything in a Job: selected words, a command, a Fleet notice, a plan task, a Check. Three repositories, each
// with a Job: the first holds everything Teach is pressed on and names its Jira site in its settings; the second's site is
// found in a pull request; the third's cannot be found, so Fleet asks. Mock only: no Fleet reads what is taught yet. The walk
// `teach-anything` plays it.

import type { LogbookAnswer, LogbookEntry, LogbookForgotten, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import type { BridgeApi } from "../../../../shared/api";
import { onBoard, unanswered } from "../moment";
import type { Scenario } from "../moment";
import { FakeSessionsFleet } from "../sessions-fleet";
import { ARMADA_JOB, BEHIND_AT, LATER_NOTICES, SITE_ASK, ledgerRecord, teachSessions, teachThreads } from "../sessions/teach";
import { bindTeachFleet, entryOf, siteAnswered } from "../teach";
import { forgetLesson, lessonsNow, taughtId } from "../../teach";

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const PAYMENTS: RepositorySummary = {
  root: "/Users/user/code/payments",
  records_root: "/records/payments",
  manifest: { ...repository().manifest!, id: "payments", repository: "payments", path: "payments/armada.yml" },
};
const LEDGER: RepositorySummary = {
  root: "/Users/user/code/ledger",
  records_root: "/records/ledger",
  manifest: { ...repository().manifest!, id: "ledger", repository: "ledger", path: "ledger/armada.yml" },
};

const AT = "2026-10-20T10:00:00.000Z";
const entry = (id: string, fields: Pick<LogbookEntry, "section" | "kind" | "text"> & Partial<LogbookEntry>): LogbookEntry => ({
  id,
  evidence: [],
  confirmed_by: 3,
  first_seen: AT,
  last_confirmed: AT,
  state: "standing",
  locked: false,
  edited: false,
  history: [],
  ...fields,
});

/** What the Logbook held before anything was taught. */
const BASE = (): LogbookEntry[] => [
  entry("lb-vitest", { section: "repo", repository: ARMADA.root, kind: "command", text: "Run `pnpm exec vitest run <touched files>`. Never the whole desktop suite." }),
  entry("lb-lease", { section: "repo", repository: ARMADA.root, kind: "gotcha", text: "Lease a slot before the first write. Writes before it are refused." }),
  entry("lb-fmt", { section: "habits", kind: "habit", text: "Never run rustfmt or cargo fmt." }),
];

function build(): Scenario {
  const board = onBoard({ repositories: [ARMADA, PAYMENTS, LEDGER] });
  const served = new FakeSessionsFleet(teachSessions(), teachThreads()).scenario(board);
  return {
    ...served,
    name: "teach-anything",
    says: "Anything in a Job taught to Armada: words, a command, a notice, a task and a Check, and what each changes",
    // A Fleet of its own for each window, so a walk told twice starts from the same Jobs.
    behaves: (handle) => {
      const fleet = new FakeSessionsFleet(teachSessions(), teachThreads());
      fleet.keepsRows = true;
      const own = fleet.scenario(board).behaves!(handle);
      bindTeachFleet({
        ask: (manifestId, repository, sites) => {
          if (manifestId !== "ledger") return;
          fleet.changed({
            ...ledgerRecord(),
            waiting_for: [
              {
                id: SITE_ASK,
                text: `Which Jira site does ${repository} use?`,
                since: new Date().toISOString(),
                source: "agent",
                act: { kind: "answer", target: "" },
                options: sites.map((label) => ({ label })),
              },
            ],
          });
        },
        // Fleet sends two more slot notices once it is taught to batch them, each later than the one taught from.
        kept: (lesson) => {
          if (lesson.answer.fleet === undefined || lesson.teaching.sessionId !== ARMADA_JOB) return;
          LATER_NOTICES.forEach(({ text, minute }, at) =>
            window.setTimeout(
              () =>
                fleet.row(ARMADA_JOB, {
                  kind: "message",
                  id: `later-${at}`,
                  at: new Date(Date.parse(BEHIND_AT) + (minute - 40) * 60_000).toISOString(),
                  from: { kind: "session", id: "fleet", title: "Fleet" },
                  text,
                  notice: "behind_base",
                }),
              600 + at * 100,
            ),
          );
        },
      });

      // The Logbook: what it held, and the entries taught since. An edit or a lock on a taught entry is held here.
      let base = BASE();
      const changed = new Map<string, Partial<LogbookEntry>>();
      const all = (): LogbookEntry[] => [...base, ...lessonsNow().flatMap((one) => entryOf(one) ?? [])].map((one) => ({ ...one, ...changed.get(one.id) }));
      const change = (id: string, how: (was: LogbookEntry) => Partial<LogbookEntry>): LogbookAnswer => {
        const was = all().find((one) => one.id === id);
        if (was === undefined) return { ok: false, outcome: unanswered(`/logbook/${id}`) };
        changed.set(id, { ...changed.get(id), ...how(was) });
        return { ok: true, entry: { ...was, ...how(was) } };
      };
      const logbook: Pick<BridgeApi, "readLogbook" | "editLogbookEntry" | "lockLogbookEntry" | "forgetLogbookEntry"> = {
        readLogbook: async () => ({ ok: true, entries: all() }),
        editLogbookEntry: async (id, text) =>
          change(id, (was) => ({ text, edited: true, history: [{ text: was.text, until: new Date().toISOString(), by: "person" as const }, ...was.history] })),
        lockLogbookEntry: async (id, locked) => change(id, () => ({ locked })),
        forgetLogbookEntry: async (id): Promise<LogbookForgotten> => {
          const lesson = lessonsNow().find((one) => taughtId(one) === id);
          // Forgetting a taught entry takes back everything the lesson changed: the note, the links and what Fleet does.
          if (lesson !== undefined) forgetLesson(lesson.id);
          else if (base.some((one) => one.id === id)) base = base.filter((one) => one.id !== id);
          else return { ok: false, outcome: unanswered(`/logbook/${id}/forget`) };
          return { ok: true };
        },
      };

      return {
        ...own,
        ...logbook,
        // The person's answer to Fleet's question is the repository's site.
        answerWaiting: async (answer) => {
          const done = await own.answerWaiting!(answer);
          if (answer.item_id === SITE_ASK) {
            const options = ["acme.atlassian.net", "acme-payments.atlassian.net"];
            const host = answer.text ?? options[answer.choice ?? 0];
            if (host !== undefined) siteAnswered("ledger", host.replace(/^https?:\/\//, ""));
          }
          return done;
        },
      };
    },
  };
}

export const teachAnything: Scenario = build();
