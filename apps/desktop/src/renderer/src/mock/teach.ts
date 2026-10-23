// Mock-only: Fleet's reads of what the owner taught, answered from a few canned inputs. A Jira key becomes a pattern for Bridge and a
// note for agents; a notice taught to batch becomes a rule for Fleet and a note; any other words are read as a note alone. Fleet
// also finds each repository's Jira site: from its settings, else from its recent pull requests, else it asks once. And the
// Logbook entries a lesson becomes, which the scenario's Logbook serves beside its own.

import type { LogbookEntry } from "@armada/protocol";
import { TEACH_SOURCE } from "@armada/components";

import { lessonsNow, saveLesson, taughtId } from "../teach";
import type { Lesson, Site, TeachAnswer, TeachSource } from "../teach";
import { ARMADA_JOB, ARMADA_TITLE, BEHIND, BEHIND_AT } from "./sessions/teach";

const READ_MS = 1200;
const FIND_MS = 400;

const JIRA_KEY = /\b[A-Z][A-Z0-9]+-\d+\b/;
export const JIRA_KEY_PATTERN = "[A-Z][A-Z0-9]+-\\d+";

/** What Fleet finds for a repository without asking: the Manifest names the first, a pull request the second. The third has neither. */
const SITES: Readonly<Record<string, Site>> = {
  armada: { address: "https://acme.atlassian.net", from: { kind: "settings" } },
  payments: { address: "https://acme-payments.atlassian.net", from: { kind: "pull_request", number: 2712 } },
};

/** What the scenario's Fleet does when a lesson is kept or a site cannot be found. */
export type TeachFleet = {
  /** Fleet could not find the repository's site: it puts the question to the person, with the sites it knows as the options. */
  ask: (manifestId: string, repository: string, sites: readonly string[]) => void;
  /** A lesson was kept. */
  kept: (lesson: Lesson) => void;
};

let fleet: TeachFleet | undefined;
let heard = new Set<(manifestId: string, site: Site) => void>();

/** The scenario's Fleet registers itself as its window starts. */
export function bindTeachFleet(next: TeachFleet | undefined): void {
  fleet = next;
}

/** Fleet found a repository's site on its own, in a pull request, say. */
export function siteFound(manifestId: string, site: Site): void {
  heard.forEach((listen) => listen(manifestId, site));
}

/** The person answered Fleet's question about a repository's site. */
export function siteAnswered(manifestId: string, host: string): void {
  heard.forEach((listen) => listen(manifestId, { address: `https://${host}`, from: { kind: "you" } }));
}

const sentence = (words: string): string => {
  const said = words.trim().replace(/\s+/g, " ");
  const first = said.charAt(0).toUpperCase() + said.slice(1);
  return /[.!?]$/.test(first) ? first : `${first}.`;
};

/** What Fleet makes of the words, by what they were said about. */
export function understood({ teaching, say, reach }: { teaching: Parameters<TeachSource["read"]>[0]["teaching"]; say: string; reach: "repository" | "all" }): TeachAnswer {
  const said = say.toLowerCase();
  const first = teaching.text.split(" ")[0] ?? "";
  if (teaching.kind === "words" && (JIRA_KEY.test(teaching.text) || said.includes("jira"))) {
    const example = JIRA_KEY.exec(teaching.text)?.[0] ?? "STORE-1234";
    const site = SITES[teaching.manifestId ?? ""];
    return {
      mod: "jira-keys",
      agents: `Tickets are Jira keys such as ${example}. Cite one as a link to the repository's Jira site.`,
      bridge: { name: "Jira key", pattern: JIRA_KEY_PATTERN, example, ...(site === undefined ? {} : { site }) },
    };
  }
  if (teaching.kind === "notice" && /batch|group|digest/.test(said)) {
    const scope = said.includes("this job") ? "job" : reach === "all" ? "all" : "repository";
    return {
      mod: "slot-behind-batched",
      agents: "Slot behind main notices arrive batched. One that blocks a merge is called out on its own.",
      fleet: { name: "Slot behind main", match: "\\d+ commits behind main", does: "batch", says: "Batch, and tell only when it blocks a merge", scope },
    };
  }
  if (teaching.kind === "command" && said.includes("pnpm")) return { agents: "This repository uses pnpm. Run `pnpm install`, never `npm install`." };
  if (teaching.kind === "check" && said.includes("flaky")) return { agents: `\`${first}\` is flaky. Rerun it once before digging into a failure.` };
  if (teaching.kind === "task" && said.includes("walk")) return { agents: "Walk a visual change before building it." };
  return { agents: sentence(say) };
}

export function createMockTeaching(): TeachSource {
  heard = new Set();
  return {
    read: (input) => new Promise((resolve) => window.setTimeout(() => resolve(understood(input)), READ_MS)),
    site: (manifestId, repository) =>
      new Promise((resolve) =>
        window.setTimeout(() => {
          const found = SITES[manifestId];
          if (found === undefined) fleet?.ask(manifestId, repository, Object.values(SITES).map((one) => one.address.replace("https://", "")));
          resolve(found);
        }, FIND_MS),
      ),
    onSite: (listen) => {
      heard.add(listen);
      return () => void heard.delete(listen);
    },
    saved: (lesson) => fleet?.kept(lesson),
  };
}

/** What the evidence row names the thing Teach was pressed on, in the Logbook's own style. */
const quoted = (lesson: Lesson): string => (lesson.teaching.kind === "words" ? lesson.teaching.text : `\`${lesson.teaching.text}\``);

/** The Logbook entry a lesson with a note for agents becomes. Others change Bridge or Fleet alone and have none. */
export function entryOf(lesson: Lesson): LogbookEntry | undefined {
  const { agents } = lesson.answer;
  if (agents === undefined) return undefined;
  const everywhere = lesson.reach === "all";
  const kind = lesson.teaching.kind === "command" ? "command" : lesson.teaching.kind === "check" ? "gotcha" : lesson.teaching.kind === "task" ? "habit" : "convention";
  return {
    id: taughtId(lesson),
    section: everywhere ? "habits" : "repo",
    ...(everywhere ? {} : { repository: lesson.teaching.root ?? lesson.teaching.repository }),
    kind,
    text: agents,
    evidence: [
      {
        job_id: lesson.teaching.sessionId,
        job_title: lesson.teaching.jobTitle,
        cite: `Teach, ${TEACH_SOURCE[lesson.teaching.kind].toLowerCase()}`,
        says: quoted(lesson),
        at: lesson.at,
      },
    ],
    confirmed_by: 1,
    first_seen: lesson.at,
    last_confirmed: lesson.at,
    state: "standing",
    locked: false,
    edited: false,
    history: [],
  };
}

/** Every entry taught now. */
export const taughtEntries = (): LogbookEntry[] => lessonsNow().flatMap((one) => entryOf(one) ?? []);

/**
 * What `teach-mods` starts from: the Jira key and the batched notice already taught in the first Job, as `teach-anything` leaves
 * them, and the other two repositories' sites already known. `root` is the first repository's, which the notice was taught in.
 */
export function seedTaught(root: string): void {
  const here = { sessionId: ARMADA_JOB, jobTitle: ARMADA_TITLE, manifestId: "armada", root, repository: "armada" };
  const key = { teaching: { kind: "words" as const, text: "STORE-1234", ...here }, say: "A Jira key. Link it to our Jira", reach: "all" as const };
  saveLesson({ ...key, answer: understood(key) });
  const notice = { teaching: { kind: "notice" as const, text: BEHIND, at: BEHIND_AT, ...here }, say: "Batch these, and don't tell me unless it blocks a merge", reach: "repository" as const };
  saveLesson({ ...notice, answer: understood(notice) });
  siteFound("payments", SITES["payments"]!);
  siteAnswered("ledger", "acme-payments.atlassian.net");
}
