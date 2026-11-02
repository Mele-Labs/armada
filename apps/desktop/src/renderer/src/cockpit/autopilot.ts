// What the Cockpit draws of one repository's Autopilot, read off Fleet's record: what it says for itself
// when a cap paused it or a pass picked nothing, what it is choosing from next, and the start form's
// fields both ways. Pure, so the words and the arithmetic are checked without a window.
// #2092, #2105, `docs/concepts/autopilot.md`.

import type { Autopilot, AutopilotDecision, AutopilotPick, AutopilotQuoted, AutopilotSettings, IssueList } from "@armada/protocol";
import type { AutopilotFields, AutopilotReading } from "@armada/components";

const two = (n: number) => String(n).padStart(2, "0");

/** Millionths of a dollar, as a person reads money: $30, $12.40. */
export function dollars(micros: number): string {
  const cents = Math.round(micros / 10_000);
  return cents % 100 === 0 ? `$${cents / 100}` : `$${(cents / 100).toFixed(2)}`;
}

/** A moment as the Until field is typed and read back: 07:00, a 24-hour time of day. Not a display, so not the owner's time format. */
export function clockOf(iso: string): string {
  const at = new Date(iso);
  return `${two(at.getHours())}:${two(at.getMinutes())}`;
}

/** Which cap paused it: `Job cap reached`. */
export function cappedOf(autopilot: Autopilot): string | undefined {
  if (autopilot.state !== "paused" || autopilot.capped_by === undefined) return undefined;
  const reached: Record<NonNullable<Autopilot["capped_by"]>, string> = { sessions: "Job cap reached", spend: "spend cap reached", until: "time to stop passed" };
  return reached[autopilot.capped_by];
}

/** What to tell him about one that is not working by itself: the cap that paused it, else why its last pass picked nothing. Nothing where there is nothing to say. */
export function whyOf(autopilot: Autopilot): string | undefined {
  if (autopilot.state === "stopped") return undefined;
  return cappedOf(autopilot) ?? (autopilot.fault === undefined ? undefined : `Last pass picked nothing: ${autopilot.fault}`);
}

/** One row of Next up: its place in the order, or none where Autopilot will pass it over, and why. */
export type NextUp = { order: number | undefined; reference: string; title: string; url: string; note: string | undefined };

/** The most Next up lists, so a long backlog never draws as a long list. */
export const NEXT_UP_ROWS = 8;

/**
 * What Autopilot is choosing from, in the tracker's order: the open issues it has not picked this
 * run, numbered, then the ones it passes over with the reason. Fleet's own rules, read off what the
 * issue list carries; a Session's source issue is not on its row, so that rule is Fleet's alone.
 */
export function nextUpOf(autopilot: Autopilot, issues: IssueList | undefined): NextUp[] {
  if (issues === undefined) return [];
  const picked = new Set(autopilot.picks.map((pick) => pick.issue_reference));
  const open = issues.issues.filter((issue) => !picked.has(issue.reference));
  const choosable = open.filter((issue) => !issue.linked_pull_request).slice(0, NEXT_UP_ROWS);
  const passed = open.filter((issue) => issue.linked_pull_request).slice(0, Math.max(0, NEXT_UP_ROWS - choosable.length));
  return [
    ...choosable.map((issue, at): NextUp => ({ order: at + 1, reference: issue.reference, title: issue.title, url: issue.url, note: undefined })),
    ...passed.map((issue): NextUp => ({ order: undefined, reference: issue.reference, title: issue.title, url: issue.url, note: "has a pull request" })),
  ];
}

/** The picks it was unsure of, waiting on the person. */
export const setAsideOf = (autopilot: Autopilot): AutopilotPick[] => autopilot.picks.filter((pick) => pick.state === "set_aside");

/** Settings as the start form's fields, which Edit caps opens on too. A cap not set is blank. */
export function fieldsOf(settings: AutopilotSettings): AutopilotFields {
  return {
    atOnce: String(settings.at_once),
    until: settings.until === undefined ? "" : clockOf(settings.until),
    spend: settings.spend_cap_micros === undefined ? "" : dollars(settings.spend_cap_micros).slice(1),
    sessions: settings.session_cap === undefined ? "" : String(settings.session_cap),
  };
}

/** Fleet's reading of a sentence: the fields, and the words each was read from, keyed as the fields are. */
export function readingOf(proposal: AutopilotSettings & { quoted?: AutopilotQuoted }): AutopilotReading {
  const { at_once, until, spend_cap_micros, session_cap } = proposal.quoted ?? {};
  const quoted = { atOnce: at_once, until, spend: spend_cap_micros, sessions: session_cap };
  const said = Object.fromEntries(Object.entries(quoted).filter(([, words]) => words !== undefined && words.trim() !== ""));
  return { fields: fieldsOf(proposal), quoted: said };
}

const whole = (text: string) => (/^\d+$/.test(text.trim()) ? Number(text.trim()) : NaN);

/** The next time the person's clock reads `text` (07:00, 7am, 19:30), after `now`. */
function nextAt(text: string, now: number): string | undefined {
  const read = /^(\d{1,2})(?::(\d{2}))?\s*(am|pm)?$/i.exec(text.trim());
  if (read === null) return undefined;
  let hours = Number(read[1]);
  const minutes = read[2] === undefined ? 0 : Number(read[2]);
  const half = read[3]?.toLowerCase();
  if (half !== undefined && (hours < 1 || hours > 12)) return undefined;
  if (half === "pm" && hours < 12) hours += 12;
  if (half === "am" && hours === 12) hours = 0;
  if (hours > 23 || minutes > 59) return undefined;
  const at = new Date(now);
  at.setHours(hours, minutes, 0, 0);
  if (at.getTime() <= now) at.setDate(at.getDate() + 1);
  return at.toISOString();
}

/** The kinds of decision Fleet knows. A box checked for any other word is not sent. */
const DECISIONS: readonly AutopilotDecision[] = ["recommended", "open", "plan", "draft", "walk", "land", "unsure", "red", "reads", "writes"];

/**
 * The fields the person confirmed, as the start Fleet takes, or what is wrong with one of them.
 * `boxes` are the checked work and decisions, sent as they were; without them Fleet reads the backlog alone.
 */
export function settingsFrom(instruction: string, fields: AutopilotFields, now: number, boxes?: { work: ReadonlySet<string>; decisions: ReadonlySet<string> }): { ok: true; settings: AutopilotSettings } | { ok: false; why: string } {
  const atOnce = whole(fields.atOnce);
  if (!(atOnce >= 1)) return { ok: false, why: "At once is a whole number of Jobs, 1 or more." };
  const settings: AutopilotSettings = { instruction: instruction.trim(), at_once: atOnce };
  if (fields.until.trim() !== "") {
    const until = nextAt(fields.until, now);
    if (until === undefined) return { ok: false, why: "Until is a time of day, like 07:00." };
    settings.until = until;
  }
  if (fields.spend.trim() !== "") {
    const spend = Number(fields.spend.trim().replace(/^\$/, "").replace(/,/g, ""));
    if (!(spend > 0)) return { ok: false, why: "Spend is an amount in dollars, like 30." };
    settings.spend_cap_micros = Math.round(spend * 1_000_000);
  }
  if (fields.sessions.trim() !== "") {
    const sessions = whole(fields.sessions);
    if (!(sessions >= 1)) return { ok: false, why: "Jobs is a whole number, 1 or more, or blank for no cap." };
    settings.session_cap = sessions;
  }
  if (boxes !== undefined) {
    settings.on_autopilot = { backlog: boxes.work.has("backlog"), running: boxes.work.has("running") };
    settings.decisions = DECISIONS.filter((kind) => boxes.decisions.has(kind));
  }
  return { ok: true, settings };
}

/** The person's clock and its offset, which Fleet reads "until 7am" against: 2026-10-11T23:00:00+01:00. */
export function localNow(now: number): string {
  const at = new Date(now);
  const offset = -at.getTimezoneOffset();
  const sign = offset >= 0 ? "+" : "-";
  const zone = `${sign}${two(Math.floor(Math.abs(offset) / 60))}:${two(Math.abs(offset) % 60)}`;
  return `${at.getFullYear()}-${two(at.getMonth() + 1)}-${two(at.getDate())}T${two(at.getHours())}:${two(at.getMinutes())}:${two(at.getSeconds())}${zone}`;
}
