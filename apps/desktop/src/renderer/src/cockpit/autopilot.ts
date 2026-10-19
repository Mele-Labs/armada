// What the Cockpit draws of one repository's Autopilot, read off Fleet's record: the caps used of their
// limit, which cap paused it, what Stop does to this run's picks, what it is choosing from next, and the
// start form's fields both ways. Pure, so the words and the arithmetic are checked without a window.
// #2092, #2105, `docs/concepts/autopilot.md`.

import type { Autopilot, AutopilotPick, AutopilotQuoted, AutopilotSettings, IssueList } from "@armada/protocol";
import type { AutopilotCap, AutopilotFields, AutopilotReading } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

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

/** How long is left until `at`: 5h 41m, 12m. */
function leftUntil(at: number, now: number): string {
  const minutes = Math.max(0, Math.round((at - now) / 60_000));
  return minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h ${two(minutes % 60)}m`;
}

const share = (used: number, of: number) => (of <= 0 ? 0 : Math.min(1, used / of));

/** Each cap, used of its limit, lit where it is reached. A cap not set says so. */
export function capsOf(autopilot: Autopilot, now: number): AutopilotCap[] {
  const { at_work, at_once, spent_micros, spend_cap_micros, sessions_started, session_cap, until, capped_by, state } = autopilot;
  const live = state !== "stopped";
  const ends = until === undefined ? NaN : Date.parse(until);
  const began = Date.parse(autopilot.started_at);
  return [
    { label: "At once", value: `${at_work} of ${at_once} running`, fill: share(at_work, at_once), reached: live && at_work >= at_once },
    spend_cap_micros === undefined
      ? { label: "Spend", value: `${dollars(spent_micros)} · no cap`, fill: 0, reached: false }
      : { label: "Spend", value: `${dollars(spent_micros)} of ${dollars(spend_cap_micros)}`, fill: share(spent_micros, spend_cap_micros), reached: capped_by === "spend" || spent_micros >= spend_cap_micros },
    session_cap === undefined
      ? { label: "Jobs", value: `${sessions_started} started · no cap`, fill: 0, reached: false }
      : { label: "Jobs", value: `${sessions_started} of ${session_cap} started`, fill: share(sessions_started, session_cap), reached: capped_by === "sessions" || sessions_started >= session_cap },
    until === undefined || Number.isNaN(ends)
      ? { label: "Until", value: "no time set", fill: 0, reached: false }
      : {
          label: "Until",
          at: until,
          value: live && now < ends ? `${leftUntil(ends, now)} left` : "",
          fill: share(now - began, ends - began),
          reached: capped_by === "until" || now >= ends,
        },
  ];
}

/** Which cap paused it, as the bar's state names it: `Job cap reached`. A person's own Pause needs no naming. */
export function cappedOf(autopilot: Autopilot): string | undefined {
  if (autopilot.state !== "paused" || autopilot.capped_by === undefined) return undefined;
  const reached: Record<NonNullable<Autopilot["capped_by"]>, string> = { sessions: "Job cap reached", spend: "spend cap reached", until: "time to stop passed" };
  return reached[autopilot.capped_by];
}

/** #1, #2 and #3: each as its tracker names it. */
function listed(picks: readonly AutopilotPick[]): string {
  const names = picks.map((pick) => pick.issue_reference);
  return names.length < 2 ? (names[0] ?? "") : `${names.slice(0, -1).join(", ")} and ${names.at(-1)}`;
}

/** The part of a Session that says whether its agent has run. */
type Started = Pick<Session, "id" | "dead" | "lastTurnAt" | "turn">;

/** The Session a started pick opened, where the window holds it. */
const sessionOf = (pick: AutopilotPick, sessions: readonly Started[]) =>
  pick.state === "started" && pick.session_id !== undefined ? sessions.find((one) => one.id === pick.session_id) : undefined;

/** The picks whose Session has never run: Stop withdraws them. */
export function notStarted(autopilot: Autopilot, sessions: readonly Started[]): AutopilotPick[] {
  return autopilot.picks.filter((pick) => {
    const session = sessionOf(pick, sessions);
    return session !== undefined && session.dead === undefined && session.lastTurnAt === undefined && session.turn.state === "idle";
  });
}

/** What Stop does to this run, said before it is pressed: what is withdrawn, and what carries on. */
export function stoppingOf(autopilot: Autopilot, sessions: readonly Started[]): string {
  const waiting = notStarted(autopilot, sessions);
  const going = autopilot.picks.filter((pick) => {
    const session = sessionOf(pick, sessions);
    return session !== undefined && session.dead === undefined && !waiting.includes(pick);
  });
  if (waiting.length === 0 && going.length === 0) return "Stop ends Autopilot. Picks that have not started are withdrawn, and started Jobs carry on.";
  const withdrawn = waiting.length === 0 ? "" : ` ${listed(waiting)}, picked and not started, ${waiting.length === 1 ? "is" : "are"} withdrawn.`;
  const carry = going.length === 0 ? "" : ` ${listed(going)} ${going.length === 1 ? "keeps" : "keep"} going to the end.`;
  return `Stop ends Autopilot.${withdrawn}${carry}`;
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

/** Whether it is picking, or could pick again on Resume. */
export const isOn = (autopilot: Autopilot | undefined): autopilot is Autopilot => autopilot !== undefined && autopilot.state !== "stopped";

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

/** The fields the person confirmed, as the start Fleet takes, or what is wrong with one of them. */
export function settingsFrom(instruction: string, fields: AutopilotFields, now: number): { ok: true; settings: AutopilotSettings } | { ok: false; why: string } {
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
