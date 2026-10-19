// **The mock's stand-in for the proposer.** Dispatch hands a brief that tags a Session with `@` to the proposer, which reads the
// wait out of it: who, when, from where and what is needed. Nothing in Fleet reads a wait yet, so this reads the words by rule and
// always reads the same words the same way. It is not a language model and says nothing a rule below does not.
//
//   lands, merged, merges             the pull request lands
//   pushes, pushed, PR is up          the pull request is pushed
//   ends, finishes, is done           the Session ends
//   when <part> is done               a part of its plan is done (the part is found by its title)
//   on its branch                     starts from the pull request's branch
//
// With no condition, and no `after`, `once` or `when` before the tag, the tag is plain: it only refers to the Session. With one of those and no
// condition the Session ends. The rest of the clause after the condition is what is needed.

import type { Session, WaitReading } from "@armada/screens/src/draft/sessions";
import { defaultBase, planPartsOf } from "@armada/screens/src/session-waits";
import { taggedIn } from "../../dispatch-panel/waits";

type Found = { at: number; end: number; when: WaitReading["when"]; part?: { id: string; title: string } };

/** The part of the plan a phrase names: its title contains the phrase, or the phrase contains its title. */
function partNamed(phrase: string, session: Session): { id: string; title: string } | undefined {
  const wanted = phrase.trim().toLowerCase();
  const part = planPartsOf(session).find((one) => one.title.toLowerCase().includes(wanted) || wanted.includes(one.title.toLowerCase()));
  return part === undefined ? undefined : { id: part.id, title: part.title };
}

const capital = (words: string) => `${words.charAt(0).toUpperCase()}${words.slice(1)}`;
const tidy = (words: string) => words.replace(/\s+/g, " ").replace(/^[\s,;:.]+/, "").replace(/[\s,;:]+$/, "");

export function readWait(words: string, sessions: readonly Session[]): WaitReading | undefined {
  const tag = taggedIn(words, sessions);
  if (tag === undefined) return undefined;
  const before = words.slice(0, tag.at);
  const rest = words.slice(tag.end);

  // The earliest condition in the words after the tag wins.
  const found: Found[] = [];
  const plan = /\b(?:when|once)\s+(.+?)\s+is done\b/i.exec(rest);
  const part = plan === null ? undefined : partNamed(plan[1]!, tag.session);
  if (plan !== null && part !== undefined) found.push({ at: plan.index, end: plan.index + plan[0].length, when: "plan", part });
  for (const [when, pattern] of [
    ["lands", /\b(?:lands?|merged|merges)\b/i],
    ["pushed", /\b(?:pushes|pushed|(?:PR|pull request) is up)\b/i],
    ["ends", /\b(?:ends|finishes|is done)\b/i],
  ] as const) {
    const hit = pattern.exec(rest);
    if (hit !== null) found.push({ at: hit.index, end: hit.index + hit[0].length, when });
  }
  const condition = found.sort((a, b) => a.at - b.at)[0];
  const when = condition?.when ?? "ends";

  // `on its branch` is the base; the words that say it are not what is needed.
  const branch = /(?:\bno\b[,\s]*)?(?:\bstart\s+)?\b(?:on|from) its (?:(?:PR|pull request) )?branch\b/i;
  const base = branch.test(rest) ? "branch" : defaultBase(when);

  // What is needed: the rest of the clause after the condition (`lands its PR, let the Field expand`), or what came before the wait.
  const afterCondition = rest
    .slice(condition?.end ?? 0)
    .replace(branch, " ")
    .replace(/^\s+its (?:PR|pull request)\b/i, "");
  const ahead = before.replace(/\b(?:after|once|when|wait for|wait on)\s*$/i, "");
  const clause = tidy(afterCondition).replace(/^(?:then|so|and)\s+/i, "") || tidy(ahead);
  const needs = clause.replace(/^let\s+/i, "");
  const plain = condition === undefined && ahead === before;

  return {
    on: { id: tag.session.id, title: tag.session.title! },
    when,
    ...(condition?.part === undefined ? {} : { part: condition.part }),
    base,
    needs,
    ...(plain ? { plain: true } : {}),
    // What the proposer would name it: the first thing asked for, as a line.
    title: clause === "" ? `After ${tag.session.title!}` : capital(clause.split(/\s+and\s+/i)[0] ?? clause),
  };
}
