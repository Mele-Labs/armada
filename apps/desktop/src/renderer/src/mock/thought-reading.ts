// Helm's reading of a thought, as the mock has it. **A keyword scorer, not a model**: it reads the
// words against every live Session and running Job the window holds, whatever repository, and names
// the best match if one clears the bar. Mock-only; a Fleet that reads would replace all of it.
//
// A Session is read by its title (weighted) and its ledger: the branch and pull request titles it holds.
// A Job is read by its title.

import type { JobSummary, SessionRecord } from "@armada/protocol";
import type { RouteTarget, ThoughtReading } from "@armada/helm";

import type { ThoughtReader, ThoughtWorld } from "../thought-routing";

/** How long Helm takes to read, so the mark turning can be seen. */
const READING_MS = 1400;

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
  const repositoryOf = (manifestId: string | undefined): string =>
    world.repositories.find((one) => one.manifest?.id === manifestId)?.manifest?.repository ?? manifestId ?? "";
  const candidates: { target: RouteTarget; score: number }[] = [
    ...world.sessions
      .filter((one) => one.state === "live" && one.title !== undefined)
      .map((one) => ({
        target: {
          kind: "session" as const,
          id: one.id,
          title: one.title!,
          repository: repositoryOf(one.manifest_id),
          ...(slotOf(one) === undefined ? {} : { slot: slotOf(one)! }),
        },
        score: score(thought, one.title!, ledgerOf(one)),
      })),
    ...world.jobs
      .filter((one: JobSummary) => one.status === "running")
      .map((one) => ({
        target: { kind: "job" as const, id: one.id, title: one.title, repository: repositoryOf(one.owner_manifest_id) },
        score: score(thought, one.title, ""),
      })),
  ];
  const best = candidates.sort((a, b) => b.score - a.score)[0];
  return best !== undefined && best.score >= BAR ? { kind: "thought", match: best.target } : { kind: "thought" };
}

export const mockReader: ThoughtReader = (text, world) =>
  new Promise((resolve) => setTimeout(() => resolve(readThought(text, world)), READING_MS));
