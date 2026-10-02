// A step's gaming check, read once for the rail, the step panel and the card
// that holds a step it stopped. #1079, #1672.
//
// **Its own reading, never a tier's.** The gaming check is counted in neither
// Checks nor Judge on purpose — a flag is not a verdict, and a recount would
// make a green tier read red — so nothing here touches `gates.ts`' counts.
//
// **A flag a second reading cleared is still a flag**, and it never holds a
// step: it is drawn in the panel as cleared, with why, and nowhere else.

import { GAMING_PATTERN, type DiffLine } from "@armada/components";
import type { Diff, Flagged, JobDetail as JobWhole, StepDetail } from "@armada/protocol";

import { onlyCurrentAttempt } from "./facts";
import { drawnOf } from "./review";

/** One attempt's flags, split into those that hold the step and those cleared. */
export type FlagsRead = { held: Flagged[]; cleared: Flagged[] };

/** The newest attempt's flags, or the rows a caller already narrowed. */
export function flagsOf(step: StepDetail, rows?: readonly Flagged[]): FlagsRead {
  const flags = rows ?? onlyCurrentAttempt(step.flagged);
  return {
    held: flags.filter((flag) => flag.cleared === undefined),
    cleared: flags.filter((flag) => flag.cleared !== undefined),
  };
}

/**
 * Whether this step is the one Fleet holds on a gaming flag that nothing
 * cleared — what Overview's lead, its block and the step panel all key on.
 *
 * **Read off `stuck`, which is Fleet's own record of the stop**, and the flags
 * on the step it names. `overridden` is a person's Carry on already taken.
 */
export function heldByAFlag(whole: JobWhole | null, step: StepDetail | undefined): boolean {
  const stuck = whole?.stuck;
  return (
    step !== undefined &&
    stuck?.stopped_by === "evidence_suspect" &&
    stuck.step_id === step.step_id &&
    !step.overridden &&
    flagsOf(step).held.length > 0
  );
}

/**
 * What a flag caught, in the registry's own verb — *An assertion now asserts
 * less*. The wire spelling only where the registry has no word for it.
 */
export function flagSaid(flag: Flagged): string {
  const verb = GAMING_PATTERN[flag.pattern]?.verb;
  return verb == null ? flag.pattern : verb.charAt(0).toUpperCase() + verb.slice(1);
}

/**
 * The words *Send it back* redirects a Drone still holding the step with.
 *
 * **Composed from the flag, never left to the Drone to look up**: what the
 * pattern caught, what the check cited and what it asked, and the person's note
 * where they wrote one. A redirect is a turn in a live session, and a Drone
 * told only "the flag is right" would have to guess which flag and why.
 */
export function sentBackWords(flags: readonly Flagged[], note: string | undefined): string {
  const found = flags.map((flag) =>
    [
      `${flagSaid(flag)}.`,
      ...(flag.cited === "" ? [] : [`It cited: ${flag.cited}`]),
      ...(flag.asked === undefined ? [] : [`It asked: ${flag.asked}`]),
    ].join("\n"),
  );
  const said = note?.trim();
  return [SENT_BACK, ...found, ...(said === undefined || said === "" ? [] : [`The person's note: ${said}`])].join(
    "\n\n",
  );
}

/** How the redirect opens. */
const SENT_BACK =
  "The gaming check flagged this step, and a person read the flag and agrees with it. Change the " +
  "work so the flag no longer applies, then submit the step again.";

/** One flag's lines, located in the patch: the file and the hunk that holds them. */
export type Located = { file: string; lines: DiffLine[] };

/**
 * The hunk a flag is about, out of this Job's patch, or `undefined`.
 *
 * **Located, never guessed.** A flag with a post-image line is found by that
 * line, counted off git's own hunk header. A flag cited on a removed line has
 * a file and no line, so it is found by the words it quotes — and where no line
 * in the file carries them, nothing is drawn rather than a nearby hunk.
 */
export function hunkFor(flag: Flagged, diff: Diff, jobId: string): Located | undefined {
  const at = flag.at;
  if (at === undefined) return undefined;
  const patch = diff.state === "read" && diff.jobId === jobId ? diff.work?.patch : undefined;
  if (patch === undefined) return undefined;
  const file = drawnOf(patch, () => "").files.find((one) => one.path === at.file);
  if (file === undefined) return undefined;
  const hunks = hunksIn(file.lines);
  const line = at.line;
  const found =
    line === undefined
      ? (hunks.find((hunk) => quotes(hunk, flag.cited, "removed")) ??
        hunks.find((hunk) => quotes(hunk, flag.cited, undefined)))
      : hunks.find((hunk) => holdsLine(hunk, line));
  return found === undefined ? undefined : { file: at.file, lines: found };
}

/** A file's lines, one array per `@@` hunk, each led by its header. */
function hunksIn(lines: readonly DiffLine[]): DiffLine[][] {
  const hunks: DiffLine[][] = [];
  for (const line of lines) {
    if (line.kind === "hunk") hunks.push([line]);
    else hunks.at(-1)?.push(line);
  }
  return hunks;
}

/** Whether a hunk draws the file's post-image line `line`, counted off its header. */
function holdsLine(hunk: readonly DiffLine[], line: number): boolean {
  const header = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(hunk[0]?.text ?? "");
  if (header === null) return false;
  let at = Number(header[1]);
  for (const row of hunk.slice(1)) {
    if (row.kind === "removed" || row.text.startsWith("\\")) continue;
    if (at === line) return true;
    at += 1;
  }
  return false;
}

/** Shorter than this, a line matches too much to be the one quoted. */
const QUOTED_AT_LEAST = 8;

/**
 * Whether a line of the hunk, of `kind` where one is named, is quoted by the
 * citation — the whole line inside it, or a backticked quotation inside the
 * line. Whitespace is collapsed on both sides, since a model re-wraps what it
 * quotes.
 */
function quotes(hunk: readonly DiffLine[], cited: string, kind: DiffLine["kind"] | undefined): boolean {
  const said = collapsed(cited);
  const quoted = [...cited.matchAll(/`([^`\n]+)`/g)]
    .map((match) => collapsed(match[1] ?? ""))
    .filter((one) => one.length >= QUOTED_AT_LEAST);
  return hunk.slice(1).some((row) => {
    if (kind !== undefined && row.kind !== kind) return false;
    const content = collapsed(row.text.slice(1));
    if (content.length < QUOTED_AT_LEAST) return false;
    return said.includes(content) || quoted.some((one) => content.includes(one));
  });
}

function collapsed(text: string): string {
  return text.replace(/\s+/g, " ").trim();
}
