// Helm's reading of a thought, as the mock Fleet answers `read_thought`. **A keyword scorer, not a
// model**: it reads the words against every live Session and running Job a scenario holds, whatever
// repository, and names the best match if one clears the bar. Only a scenario about routing a thought
// answers with it; every other mock Fleet reads everything as a question (`helm-api.ts`).
//
// A Session is read by its title (weighted) and its ledger: the branch and pull request titles it holds.
// A Job is read by its title.

import type { JobSummary, SessionRecord, ThoughtRead, ThoughtReading, ThoughtTarget } from "@armada/protocol";

/** What a scenario holds that a thought could go to. */
export type ThoughtWorld = {
  sessions: readonly SessionRecord[];
  jobs: readonly JobSummary[];
};

/** How long Helm takes to read, so the mark turning can be seen. */
export const READING_MS = 1400;

/** A match needs this much overlap: two words of a title, or a title word and two of the ledger. */
const BAR = 4;

const STOP = new Set(
  "the that this with would could should into they them their there are was were for and but not you your our just new from then than when what which who how why does did have has had can will its also all any too very more some into onto about after before over under out off one two".split(" "),
);

/** Words, lower case, with a plural, a tense and a trailing e taken off, so "retries" meets "retry". */
function words(text: string): Set<string> {
  const found = new Set<string>();
  for (const raw of text.toLowerCase().match(/[a-z]+/g) ?? []) {
    if (raw.length < 3 || STOP.has(raw)) continue;
    let word = raw;
    if (word.endsWith("ies") && word.length > 4) word = `${word.slice(0, -3)}y`;
    else if (word.endsWith("s") && !word.endsWith("ss")) word = word.slice(0, -1);
    if (word.endsWith("ed") && word.length > 4) word = word.slice(0, -2);
    else if (word.endsWith("ing") && word.length > 5) word = word.slice(0, -3);
    if (word.endsWith("e") && word.length > 3) word = word.slice(0, -1);
    found.add(word);
  }
  return found;
}

function score(thought: Set<string>, title: string, ledger: string): number {
  const named = words(title);
  const held = words(ledger);
  let total = 0;
  for (const word of thought) total += named.has(word) ? 2 : held.has(word) ? 1 : 0;
  return total;
}

const QUESTION = /\?\s*$|^\s*(why|what|how|which|when|where|who|is|are|does|did|can|could)\b/i;

function ledgerOf(session: SessionRecord): string {
  return session.attachments
    .flatMap((one) => [one.kind === "branch" ? one.target : "", one.detail?.title ?? "", one.detail?.branch ?? ""])
    .join(" ");
}

function slotOf(session: SessionRecord): number | undefined {
  const slot = session.attachments.find((one) => one.kind === "slot");
  return slot === undefined ? undefined : Number(slot.target);
}

export function readThought(text: string, world: ThoughtWorld): ThoughtReading {
  if (QUESTION.test(text)) return { kind: "question" };
  const thought = words(text);
  const candidates: { target: ThoughtTarget; score: number }[] = [
    ...world.sessions
      .filter((one) => one.state === "live" && one.title !== undefined)
      .map((one) => ({
        target: {
          kind: "session" as const,
          id: one.id,
          title: one.title!,
          ...(one.manifest_id === undefined ? {} : { manifest_id: one.manifest_id }),
          ...(slotOf(one) === undefined ? {} : { slot: slotOf(one)! }),
        },
        score: score(thought, one.title!, ledgerOf(one)),
      })),
    ...world.jobs
      .filter((one) => one.status === "running")
      .map((one) => ({
        target: {
          kind: "job" as const,
          id: one.id,
          title: one.title,
          manifest_id: one.owner_manifest_id,
        },
        score: score(thought, one.title, ""),
      })),
  ];
  const best = candidates.sort((a, b) => b.score - a.score)[0];
  return best !== undefined && best.score >= BAR ? { kind: "thought", target: best.target } : { kind: "thought" };
}

/** The scenario's answer to `read_thought`, after the pause a real reading takes. */
export const readAfterPause = (text: string, world: ThoughtWorld): Promise<ThoughtRead> =>
  new Promise((resolve) => setTimeout(() => resolve({ ok: true, reading: readThought(text, world) }), READING_MS));
