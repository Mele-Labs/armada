// The message a report about a Session starts a new Session with: the owner's words as he typed them, then what Bridge holds about the
// reported Session as plain labelled lines, so the new agent starts from the facts and digs through nothing. Every line is read from
// Fleet's record or the window's state and is left out where it is not known; the agent is never asked for one. The owner's ask of 15 Oct 2026.

import type { TimelineEvent } from "@armada/protocol";
import type { Session, SessionAttachment, SessionRow } from "@armada/screens/src/draft/sessions";

/** What the window knows that the Session alone does not say. */
export type ReportFrom = {
  at: Date;
  /** Where the report was pressed: the screen and the control. */
  screen: string;
  /** The repository's root directory. */
  root: string;
  /** The repository's name, where it has one. */
  repository?: string;
  /** The timeline's entries, any subject; the Session's own are picked out here. */
  events: readonly TimelineEvent[];
  /** What Fleet last refused a Session act, in words. */
  refused?: string;
};

const MESSAGES = 6;
const TOOLS = 5;
const EVENTS = 5;
const CHARS = 300;

/** One line, cut: whitespace folded and a cut marked with an ellipsis. */
const cut = (text: string, most = CHARS): string => {
  const flat = text.replace(/\s+/g, " ").trim();
  return flat.length <= most ? flat : `${flat.slice(0, most - 1).trimEnd()}…`;
};

/** Where Claude Code keeps a session's transcript: the directory is the working directory with every character that is not a letter or digit as `-`. */
export const transcriptOf = (cwd: string, id: string): string => `~/.claude/projects/${cwd.replace(/[^A-Za-z0-9]/g, "-")}/${id}.jsonl`;

const kb = (tokens: number) => `${Math.round(tokens / 1000)}k`;

/** Its state in the words the Sessions page uses for what it can be sent. */
function stateOf(session: Session): string {
  const base = session.dead === "ended" ? "ended" : session.dead === "quiet" ? "terminal not listening" : session.turn.state === "working" ? "working" : "idle";
  if (session.standing === undefined || session.standing === base) return base;
  const running = base === "idle" && session.standing === "working" ? session.attachments.find((one) => one.kind === "subagent" && one.state === "running") : undefined;
  if (running?.kind === "subagent") return `${base}, a subagent working: ${running.task}`;
  return `${base}, read by Fleet as ${session.standing}`;
}

function attachmentLine(one: SessionAttachment): [label: string, value: string] | undefined {
  switch (one.kind) {
    case "pull_request": {
      const checks = one.checks.state === "failed" ? `checks failed (${one.checks.failing})` : `checks ${one.checks.state}`;
      return ["Pull request", `#${one.number} ${one.title}, ${one.state}, ${checks}, branch ${one.branch}, ${one.address}`];
    }
    case "forked_from":
      return ["Forked from Job", one.id];
    case "forked_to":
      return ["Forked to Job", one.id];
    case "started_by":
      return ["Started by Job", one.id];
    case "started_to":
      return ["Started Job", one.id];
    default:
      return undefined;
  }
}

const who = (row: Extract<SessionRow, { kind: "message" }>) => (row.from.kind === "you" ? "Owner" : row.from.kind === "agent" ? "Agent" : `Job ${row.from.title} (${row.from.id})`);

/** The labelled lines about `session`, one per fact, in the order they are read. A fact that is not known is not a line. */
export function reportFacts(session: Session, from: ReportFrom): string[] {
  const lines: string[] = [];
  const add = (label: string, value: string | undefined) => void (value === undefined || value === "" || lines.push(`${label}: ${value}`));
  add("Reported at", from.at.toISOString());
  add("Pressed from", from.screen);
  add("Job", session.title === undefined ? undefined : `${session.title} (${session.id})`);
  add("Armada job id", session.id);
  add("Claude Code session id", session.id);
  add("Kind", session.terminal === true ? "run in a terminal" : "hosted by Fleet");
  add("Repository", from.repository === undefined ? from.root : `${from.repository}, ${from.root}`);
  if (session.cwd !== undefined) {
    add("Started in", session.cwd);
    add("Transcript", transcriptOf(session.cwd, session.id));
  }
  const branches = session.attachments.flatMap((one) => (one.kind === "branch" ? [one] : []));
  const slots = session.attachments.flatMap((one) => (one.kind === "slot" ? [one] : []));
  const bayPath = (slot: number) => `${from.root}/.armada/slots/slot-${slot}`;
  for (const one of branches) add("Branch", `${one.name}, slot ${one.slot}, ${bayPath(one.slot)}`);
  if (branches.length === 0) for (const one of slots) add("Slot", `${one.slot}, ${bayPath(one.slot)}`);
  if (branches.length === 0 && slots.length === 0) add("Slot held earlier", (session.trail?.bays ?? []).map((one) => `slot-${one.slot}`).join(", "));
  add("State", stateOf(session));
  add("Started", session.trail?.started);
  add("Last turn ended", session.lastTurnAt);
  add("Held on the plan limit until", session.limitedUntil);
  const model = [session.model, session.resolvedModel === undefined ? undefined : `resolved ${session.resolvedModel}`, session.effort === undefined ? undefined : `effort ${session.effort}`, session.mode === undefined ? undefined : `mode ${session.mode}`].filter((one) => one !== undefined);
  add("Model", model.length === 0 ? undefined : model.join(", "));
  add("Context", session.context === undefined ? undefined : `${kb(session.context.tokens)} of ${kb(session.context.window)} tokens`);
  for (const one of session.attachments) {
    const line = attachmentLine(one);
    if (line !== undefined) add(line[0], line[1]);
  }
  for (const item of session.waitingFor ?? []) add("Waiting on the owner", `${cut(item.text, 200)} (${item.source}, since ${item.since})`);
  if (session.asked !== undefined) add("Held on a call", cut(session.asked.command, 200));
  if (session.lapsed !== undefined) add("Question nobody answered", cut(session.lapsed.questions.map((one) => one.question).join(" / "), 200));
  add("Last Job act Fleet refused", from.refused === undefined ? undefined : cut(from.refused, 200));
  return lines;
}

/** The sections that are lists: its last messages, its last tool calls and the timeline's last entries for it. */
export function reportLists(session: Session, from: ReportFrom): string[] {
  const out: string[] = [];
  const section = (head: string, rows: readonly string[]) => void (rows.length === 0 || out.push("", `${head}:`, ...rows.map((one) => `- ${one}`)));
  const messages = session.rows.flatMap((row) => (row.kind === "message" ? [`${who(row)} at ${row.at}: ${cut(row.text)}`] : []));
  section("Last messages, oldest first", messages.slice(-MESSAGES));
  const tools = session.rows.flatMap((row) => (row.kind === "tool" ? [`${row.at} ${cut(row.text, 160)}`] : []));
  section("Last tool calls, oldest first", tools.slice(-TOOLS));
  const events = [...from.events]
    .filter((one) => one.subject.kind === "session" && one.subject.id === session.id)
    .sort((a, b) => Date.parse(a.at) - Date.parse(b.at) || a.id - b.id)
    .slice(-EVENTS)
    .map((one) => `${one.at} ${one.kind}: ${cut(one.says, 160)}${one.detail === undefined ? "" : ` (${cut(one.detail, 120)})`}${one.tone === "bad" ? ", failing" : ""}`);
  section("Last timeline events, oldest first", events);
  return out;
}

/** The owner's words first and untouched, then the facts, then what to do with them. */
export function reportBrief(words: string, session: Session, from: ReportFrom): string {
  return [
    words.trim(),
    "",
    "The owner reported the Job below from Bridge. What follows is what Bridge held about it when he pressed Report.",
    "",
    ...reportFacts(session, from),
    ...reportLists(session, from),
    "",
    "Read its transcript and its Fleet record (list_sessions, get_events_since), work out what happened, and say what you found. If the fix is small, make it on a branch. If it is not small, file an issue.",
  ].join("\n");
}
