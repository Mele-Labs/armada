// Mock-only: Fleet's reads and keeps of what the owner taught, answered from a few canned inputs, over the wire the real Fleet speaks
// (`readTeaching`, `saveTeaching`, `listWordRules`, and the mod list). A Jira key becomes a Words mod and a note for agents; a notice
// taught to batch becomes a Fleet mod and a note; a row taught to be louder becomes a theme tweak and a panel taught to go becomes a
// layout change, neither with a note; any other words are read as a note alone. Fleet also finds each repository's Jira site: from its
// settings, else from its recent pull requests, else it asks once, when the rules for that repository are listed.
//
// It answers as Fleet does and no more: a notice by its word, an address with where it was found, a mod with the entry it came from,
// and what each mod holds in the file `validate_mod` checks. And the Logbook entries a lesson becomes, which the scenario's Logbook
// serves beside its own.

import type { FleetRule, LogbookEntry, ModChecked, ModSummary, Outcome, ReadTeaching, SaveTeaching, TeachingProposal, TeachingRead, TeachingSaved, WordRule, WordRulesRead } from "@armada/protocol";
import { TEACH_SOURCE } from "@armada/components";
import { LAYOUT, parseLayout } from "@armada/shell";

import type { BridgeState } from "../../../shared/bridge";
import { BRIDGE, forgetLesson, lessonsNow, saveLesson, taughtId } from "../teach";
import type { LayoutChange, Lesson, Site, SiteFrom, TeachAnswer, Teaching } from "../teach";
import { fleetAlso } from "../taught-parts";
import { mockLayout, TIDY_DASHBOARD } from "./layout";
import { unanswered } from "./moment";
import { ARMADA_JOB, ARMADA_TITLE, BEHIND, BEHIND_AT, LEDGER_JOB, LEDGER_TITLE, PAYMENTS_JOB, PAYMENTS_TITLE } from "./sessions/teach";
import { LIGHTER, LOUDER } from "./teach-colours";
import { DUSK_MOD, mockThemes } from "./themes";

const READ_MS = 1200;
const FIND_MS = 400;

const JIRA_KEY = /\b[A-Z][A-Z0-9]+-\d+\b/;
export const JIRA_KEY_PATTERN = "[A-Z][A-Z0-9]+-\\d+";

/** What Fleet finds for a repository without asking: the Manifest names the first, a pull request the second. The third has neither. */
const SITES: Readonly<Record<string, Site>> = {
  armada: { address: "https://acme.atlassian.net", from: { kind: "settings" } },
  payments: { address: "https://acme-payments.atlassian.net", from: { kind: "pull_request", number: 2712 } },
};

/** The Jobs `teach-anything` opens, by the repository each is in. */
const JOBS: Readonly<Record<string, { manifestId: string; title: string }>> = {
  [ARMADA_JOB]: { manifestId: "armada", title: ARMADA_TITLE },
  [PAYMENTS_JOB]: { manifestId: "payments", title: PAYMENTS_TITLE },
  [LEDGER_JOB]: { manifestId: "ledger", title: LEDGER_TITLE },
};

/** What the scenario's Fleet does when a lesson is kept or a site cannot be found. */
export type TeachFleet = {
  /** Fleet could not find the repository's site: it puts the question to the person, with the sites it knows as the options. */
  ask: (manifestId: string, repository: string, sites: readonly string[]) => void;
  /** A lesson was kept. */
  kept: (lesson: Lesson) => void;
};

/** What publishes the mod list: the window's Fleet. */
export type TeachHandle = { state: () => BridgeState; publish: (change: Partial<BridgeState>) => void };

let fleet: TeachFleet | undefined;
let handle: TeachHandle | undefined;
/** The sites found or answered, by Manifest. */
let sites: Record<string, Site> = {};
let asked = new Set<string>();
/** The mods this machine switched off. */
let off = new Set<string>();
/** Bumped when an address is found, so a Words mod's file reads as written again. */
let written = 0;
/** What each proposal was understood as, by the proposal: the wire carries the parts and this keeps the words for them. */
const understanding = new Map<string, TeachAnswer>();

/** The scenario's Fleet registers itself as its window starts, with the handle that publishes the mod list. */
export function bindTeachFleet(next: TeachFleet | undefined, on?: TeachHandle): void {
  fleet = next;
  handle = on;
  sites = {};
  asked = new Set();
  off = new Set();
  written = 0;
  understanding.clear();
}

/** Fleet found a repository's site on its own, in a pull request, say. */
export function siteFound(manifestId: string, site: Site): void {
  sites = { ...sites, [manifestId]: site };
  written += 1;
  publishMods();
}

/** The person answered Fleet's question about a repository's site. */
export function siteAnswered(manifestId: string, host: string): void {
  siteFound(manifestId, { address: `https://${host}`, from: { kind: "you" } });
}

/** A panel taught away: the layout `hide-<panel>` names it, in the region that registers the panel's name, lowercased. */
const hiding = (panel: string): LayoutChange => {
  const id = panel.toLowerCase();
  const region = LAYOUT["job.panels"].entries.some((one) => one.id === id) ? "job.panels" : "dashboard.panels";
  return { mod: `hide-${id}`, region, hidden: [id] };
};

const sentence = (words: string): string => {
  const said = words.trim().replace(/\s+/g, " ");
  const first = said.charAt(0).toUpperCase() + said.slice(1);
  return /[.!?]$/.test(first) ? first : `${first}.`;
};

/** What Fleet makes of the words, by what they were said about. */
export function understood({ teaching, say, reach }: { teaching: Teaching; say: string; reach: "repository" | "all" }): TeachAnswer {
  const said = say.toLowerCase();
  const first = teaching.text.split(" ")[0] ?? "";
  if (teaching.kind === "words" && (JIRA_KEY.test(teaching.text) || said.includes("jira"))) {
    const example = JIRA_KEY.exec(teaching.text)?.[0] ?? "STORE-1234";
    const site = SITES[teaching.manifestId ?? ""];
    return {
      agents: `Tickets are Jira keys such as ${example}. Cite one as a link to the repository's Jira site.`,
      bridge: { mod: "jira-keys", name: "Jira key", pattern: JIRA_KEY_PATTERN, example, ...(site === undefined ? {} : { site }) },
    };
  }
  if (teaching.kind === "notice" && /batch|group|digest/.test(said)) {
    const scope = said.includes("this job") ? "job" : reach === "all" ? "all" : "repository";
    return {
      agents: "Slot behind main notices arrive batched. One that blocks a merge is called out on its own.",
      fleet: { mod: "slot-behind-batched", name: "Slot behind main", match: "\\d+ commits behind main", does: "batch", when: [{ if: "blocks_merge", does: "tell" }], scope },
    };
  }
  if (teaching.kind === "row" && /louder|bolder|brighter|stand out/.test(said)) return { tweak: LOUDER };
  if (teaching.kind === "panel" && /hide|remove|never look/.test(said)) return { layout: hiding(teaching.text) };
  if (teaching.kind === "command" && said.includes("pnpm")) return { agents: "This repository uses pnpm. Run `pnpm install`, never `npm install`." };
  if (teaching.kind === "check" && said.includes("flaky")) return { agents: `\`${first}\` is flaky. Rerun it once before digging into a failure.` };
  if (teaching.kind === "task" && said.includes("walk")) return { agents: "Walk a visual change before building it." };
  return { agents: sentence(say) };
}


// -- What Fleet makes of a lesson, as the wire has it -------------------------------------------------------------------------

/** What a lesson was taught on, in the words a mod's row says. */
function taughtOn(teaching: Teaching): string {
  const text = teaching.text.split("\n")[0]!.trim();
  const named = text.length > 60 ? `${text.slice(0, 59)}…` : text;
  switch (teaching.kind) {
    case "words":
      return named;
    case "panel":
      return `the ${named} panel`;
    case "pull_request":
      return `the pull request ${named}`;
    case "title_bar":
      return `the title bar ${named}`;
    default:
      return `the ${TEACH_SOURCE[teaching.kind].toLowerCase()} ${named}`;
  }
}

/** What a Words mod does, in one line. */
function wordsDo(lesson: Lesson): string {
  const { bridge } = lesson.answer;
  return `${bridge!.name}s link to ${lesson.reach === "all" ? "each repository's" : "this repository's"} site`;
}

/** What a Fleet mod does, in one line, in the words its Logbook entry's `Also changes` uses. */
function fleetDoes(lesson: Lesson): string {
  const said = fleetAlso({ rule: [ruleOf(lesson.answer.fleet!, lesson.teaching)] })!;
  return `${said.charAt(0).toUpperCase()}${said.slice(1)}`;
}

const noteKind = (kind: Teaching["kind"]): LogbookEntry["kind"] => (kind === "command" ? "command" : kind === "check" ? "gotcha" : kind === "task" ? "habit" : "convention");

/** The proposal Fleet reads a lesson into. The wire names a notice by its word and a layout by its file; the words stay in `understanding`. */
function proposalOf(answer: TeachAnswer, teaching: Teaching, reach: "repository" | "everywhere"): TeachingProposal {
  const { agents, bridge, fleet: rule, tweak, layout } = answer;
  return {
    ...(agents === undefined ? {} : { note: { section: reach === "everywhere" ? ("habits" as const) : ("repo" as const), kind: noteKind(teaching.kind), text: agents } }),
    ...(bridge === undefined ? {} : { words: { name: bridge.name, pattern: bridge.pattern, example: bridge.example, ...(bridge.site === undefined ? {} : { address: { template: `${bridge.site.address}/browse/{match}`, found: foundOf(bridge.site.from) } }) } }),
    ...(rule === undefined ? {} : { fleet: ruleOf(rule, teaching) }),
    ...(tweak === undefined ? {} : { tweak: [{ token: tweak.token, was: tweak.was, now: tweak.now, used_in: [...tweak.places], note: tweak.colour }] }),
    ...(layout === undefined ? {} : { layout: { name: layout.mod, layout: layoutText(layout) } }),
  };
}

/** The rule as the wire has it: a notice by its word, and one string for where. */
const ruleOf = (rule: NonNullable<TeachAnswer["fleet"]>, teaching: Teaching): FleetRule => ({
  notice: "behind_base",
  matches: rule.match,
  does: rule.does,
  ...(rule.when === undefined ? {} : { when: rule.when }),
  where: rule.scope === "all" ? "everywhere" : rule.scope === "job" ? `job ${teaching.sessionId}` : `repo ${teaching.manifestId ?? ""}`,
});

/** How the wire spells where an address was found. */
const foundOf = (from: SiteFrom): string => (from.kind === "settings" ? "settings" : from.kind === "pull_request" ? `pull_request #${from.number}` : "person");

const layoutText = (layout: LayoutChange): string => JSON.stringify({ version: 1, [layout.region]: { hidden: layout.hidden } });

/** A proposal's identity: the same parts are the same understanding. */
const keyOf = ({ note, words, fleet: rule, tweak, layout }: TeachingProposal): string => JSON.stringify([note, words, rule, tweak, layout]);

/** The teaching a wire body names. A Job is told by its id; Bridge's own screen has none. */
function teachingOf(body: Pick<ReadTeaching, "source" | "session_id" | "repository"> & { words?: string }): Teaching {
  const source = body.source ?? { kind: "words" as const, text: body.words ?? "" };
  const job = body.session_id === undefined ? undefined : JOBS[body.session_id];
  return {
    kind: source.kind,
    text: body.words ?? source.text,
    ...(source.kind === "notice" ? { at: BEHIND_AT } : {}),
    sessionId: body.session_id ?? BRIDGE,
    jobTitle: job?.title ?? "Bridge",
    ...(job === undefined ? {} : { manifestId: job.manifestId }),
    ...(body.repository === undefined ? {} : { root: body.repository }),
    repository: job?.manifestId ?? "Bridge",
  };
}

/** The Manifest a repository's root belongs to. */
function manifestOf(root: string): string {
  return state()?.holds.repositories?.find((one) => one.root === root)?.manifest?.id ?? root.split("/").pop()!;
}

const state = (): BridgeState | undefined => handle?.state();

/** `read_teaching`: one reading, after a moment, and nothing kept. */
export function readTeaching(body: ReadTeaching): Promise<TeachingRead> {
  if (handle === undefined) return Promise.resolve({ ok: false, outcome: unanswered("/teachings/read") });
  return new Promise((resolve) =>
    window.setTimeout(() => {
      const teaching = teachingOf(body);
      const answer = understood({ teaching, say: body.said, reach: body.reach === "everywhere" ? "all" : "repository" });
      const proposal = proposalOf(answer, teaching, body.reach);
      understanding.set(keyOf(proposal), answer);
      resolve({ ok: true, value: proposal });
    }, READ_MS),
  );
}

/** The mods a lesson wrote, by name. */
const modsOf = ({ bridge, fleet: rule, tweak, layout }: TeachAnswer): string[] => [bridge?.mod, rule?.mod, tweak?.mod, layout?.mod].flatMap((one) => (one === undefined ? [] : [one]));

/** A lesson is kept: Fleet writes its mods, finds the address, and Bridge hears of the mods as it would of any change. */
function keep(input: Omit<Lesson, "id" | "at">, live: boolean): Lesson {
  const lesson = saveLesson(input);
  const { bridge, layout } = lesson.answer;
  const manifestId = lesson.teaching.manifestId;
  if (bridge?.site !== undefined && manifestId !== undefined) sites = { ...sites, [manifestId]: bridge.site };
  if (live && layout !== undefined) mockLayout.install({ name: layout.mod, title: layout.mod, enabled: true, file: parseLayout(layoutText(layout)).file });
  publishMods();
  if (live) fleet?.kept(lesson);
  return lesson;
}

/** `save_teaching`: the proposal as it was shown. */
export function saveTeaching(body: SaveTeaching): Promise<TeachingSaved> {
  const answer = understanding.get(keyOf(body));
  if (handle === undefined || answer === undefined) return Promise.resolve({ ok: false, outcome: unanswered("/teachings") });
  const lesson = keep({ reach: body.reach === "everywhere" ? "all" : "repository", teaching: teachingOf({ source: body.source, session_id: body.session_id, repository: body.repository }), say: "", answer }, true);
  const mods = modsOf(answer);
  const entry = answer.agents === undefined ? undefined : taughtId(lesson);
  return Promise.resolve({ ok: true, value: { ...(entry === undefined ? {} : { entry }), ...(mods.length === 0 ? {} : { mods }) } });
}

/** Forget on a taught Logbook entry takes back everything the lesson changed: the note, the links and what Fleet does. */
export function forgetTaught(lessonId: string): void {
  forgetLesson(lessonId);
  publishMods();
}

// -- The rules, and the mod list ----------------------------------------------------------------------------------------------

/** The lessons whose words reach a repository, one for each Words mod. */
function wordsReaching(manifestId: string): Lesson[] {
  const seen = new Set<string>();
  return lessonsNow().filter((one) => {
    const mod = one.answer.bridge?.mod;
    if (mod === undefined || seen.has(mod) || off.has(mod) || !(one.reach === "all" || one.teaching.manifestId === manifestId)) return false;
    seen.add(mod);
    return true;
  });
}

const addressIn = (site: Site): string => `${site.address}/browse/{match}`;

/**
 * `list_word_rules`: the patterns with an address for this repository. A repository whose address is not known has it looked for,
 * off this request: from its settings or a pull request, else Fleet asks. The rule is listed on a later read.
 */
export function listWordRules(root: string | undefined): Promise<WordRulesRead> {
  if (handle === undefined) return Promise.resolve({ ok: false, outcome: unanswered("/teachings/words") });
  const manifestId = root === undefined ? undefined : manifestOf(root);
  const rules: WordRule[] = [];
  for (const one of manifestId === undefined ? [] : wordsReaching(manifestId)) {
    const bridge = one.answer.bridge!;
    const site = sites[manifestId!];
    if (site === undefined) {
      if (!asked.has(manifestId!)) {
        asked.add(manifestId!);
        const id = manifestId!;
        window.setTimeout(() => {
          const found = SITES[id];
          if (found !== undefined) siteFound(id, found);
          else fleet?.ask(id, id, Object.values(SITES).map((each) => each.address.replace("https://", "")));
        }, FIND_MS);
      }
      continue;
    }
    rules.push({ mod_name: bridge.mod, name: bridge.name, pattern: bridge.pattern, example: bridge.example, address: addressIn(site), found: foundOf(site.from) });
  }
  return Promise.resolve({ ok: true, value: { rules } });
}

/** The mod list Fleet publishes: one row for each mod the lessons made. */
function modRows(): ModSummary[] {
  const seen = new Set<string>();
  const rows: ModSummary[] = [];
  const add = (row: Omit<ModSummary, "enabled" | "valid">) => {
    if (seen.has(row.name)) return;
    seen.add(row.name);
    rows.push({ ...row, enabled: !off.has(row.name), valid: true });
  };
  for (const lesson of lessonsNow()) {
    const { bridge, fleet: rule, tweak, layout } = lesson.answer;
    // Only a teaching with a note made an entry for the Logbook, and its mods say which.
    const made = lesson.answer.agents === undefined ? {} : { entry: taughtId(lesson) };
    const taught_on = taughtOn(lesson.teaching);
    const at = Date.parse(lesson.at);
    if (bridge !== undefined) add({ ...made, name: bridge.mod, kind: "words", description: wordsDo(lesson), taught_on, changed_at: new Date(at + written).toISOString() });
    if (rule !== undefined) add({ ...made, name: rule.mod, kind: "fleet", description: fleetDoes(lesson), taught_on, changed_at: new Date(at).toISOString() });
    if (tweak !== undefined) add({ ...made, name: tweak.mod, kind: "tweak", description: `${tweak.colour}, ${tweak.now}`, taught_on, changed_at: new Date(at).toISOString() });
    if (layout !== undefined) add({ ...made, name: layout.mod, kind: "layout", taught_on, changed_at: new Date(at).toISOString() });
  }
  return rows.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : 0));
}

function publishMods(): void {
  handle?.publish({ mods: { mods: modRows() } });
}

/** The lesson that wrote a mod, and what part of it the mod is. */
function lessonOf(name: string): Lesson | undefined {
  return lessonsNow().find((one) => modsOf(one.answer).includes(name));
}

// -- The mods' own routes, which a scenario that teaches adds to its Fleet's --------------------------------------------------

const OK: Outcome = { ok: true };

/** `validate_mod`, `set_mod_enabled` and `promote_mod` for the mods the lessons made. */
export const teachedMods = {
  validateMod: async (name: string): Promise<ModChecked | null> => {
    const lesson = lessonOf(name);
    if (lesson === undefined) return null;
    const { bridge, fleet: rule, tweak, layout } = lesson.answer;
    if (bridge?.mod === name) {
      const reached = Object.entries(sites).filter(([manifestId]) => lesson.reach === "all" || manifestId === lesson.teaching.manifestId);
      const address = reached.map(([repository, site]) => ({ repository, template: addressIn(site), found: foundOf(site.from) }));
      return { name, valid: true, problems: [], words: { name: bridge.name, pattern: bridge.pattern, example: bridge.example, ...(address.length === 0 ? {} : { address }) } };
    }
    if (rule?.mod === name) return { name, valid: true, problems: [], fleet: { rule: [ruleOf(rule, lesson.teaching)] } };
    if (tweak?.mod === name) return { name, valid: true, problems: [], css: `:root { ${tweak.token}: ${tweak.now}; }` };
    if (layout?.mod === name) return { name, valid: true, problems: [], layout: layoutText(layout) };
    return { name, valid: true, problems: [] };
  },
  setModEnabled: async (name: string, enabled: boolean): Promise<Outcome> => {
    off = new Set(enabled ? [...off].filter((one) => one !== name) : [...off, name]);
    publishMods();
    return OK;
  },
  tryTrackerMod: async (): Promise<Outcome> => OK,
  promoteMod: async (name: string): Promise<Outcome> => ({ ok: true, modPromoted: { name, branch: `armada/mod-${name}-${Date.now()}`, commit: "abc1234" } }),
};

/** What the evidence row names the thing Teach was pressed on, in the Logbook's own style. */
const quoted = (lesson: Lesson): string => (lesson.teaching.kind === "words" ? lesson.teaching.text : `\`${lesson.teaching.text}\``);

/** The Logbook entry a lesson with a note for agents becomes. Others change Bridge or Fleet alone and have none. */
export function entryOf(lesson: Lesson): LogbookEntry | undefined {
  const { agents } = lesson.answer;
  if (agents === undefined) return undefined;
  const everywhere = lesson.reach === "all";
  return {
    id: taughtId(lesson),
    section: everywhere ? "habits" : "repo",
    ...(everywhere ? {} : { repository: lesson.teaching.root ?? lesson.teaching.repository }),
    kind: noteKind(lesson.teaching.kind),
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
  const taught = (input: Omit<Lesson, "id" | "at">) => void keep(input, false);
  const key = { teaching: { kind: "words" as const, text: "STORE-1234", ...here }, say: "A Jira key. Link it to our Jira", reach: "all" as const };
  taught({ ...key, answer: understood(key) });
  const notice = { teaching: { kind: "notice" as const, text: BEHIND, at: BEHIND_AT, ...here }, say: "Batch these, and don't tell me unless it blocks a merge", reach: "repository" as const };
  taught({ ...notice, answer: understood(notice) });
  siteFound("payments", SITES["payments"]!);
  siteAnswered("ledger", "acme-payments.atlassian.net");
  // And one mod of each other kind, so the list of every mod on the machine has all five: a tweak taught on a row, a layout taught on
  // the rail's Guides, and a theme mod in force.
  const row = { ...here, kind: "row" as const, text: PAYMENTS_TITLE };
  taught({ teaching: row, say: "Make stopped Jobs easier to see", reach: "all", answer: { tweak: LIGHTER } });
  const panel = { ...here, kind: "panel" as const, text: "Guides" };
  taught({ teaching: panel, say: "Hide this, I never look at it", reach: "all", answer: { layout: { mod: "hide-guides", region: "rail", hidden: ["guides"] } } });
  mockThemes.install(DUSK_MOD);
  mockThemes.setActive(DUSK_MOD.name);
  // A layout mod written by hand and switched off, which the list keeps under those that are on.
  mockLayout.install({ name: "tidy", title: "Tidy", enabled: false, file: parseLayout(TIDY_DASHBOARD).file });
}
