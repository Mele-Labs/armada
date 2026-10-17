// The wait a brief names, as Dispatch draws and sends it. The proposer reads the brief (`SessionsDraft.readsWait`) and the sentence over the
// request shows what it read; a part the person changed by hand is his until the words say something else about it. A hand is kept with what
// the words read when he made it (`was`), so a correction typed in the brief that changes the reading takes the part back and one that
// leaves it alone does not undo his change.

import type { StartBase, StartWhen, WaitReading } from "@armada/screens/src/draft/sessions";
import type { PlanPart } from "@armada/screens/src/session-waits";

/** The four parts of the sentence. */
export type WaitPart = "on" | "when" | "base" | "needs";

export type Hand = {
  /** The Session picked on the sentence, which is the tag now in the words. */
  on?: { id: string; was: string };
  when?: { key: string; was: string };
  base?: { value: StartBase; was: StartBase };
  needs?: { value: string; was: string };
};

export type Option = { value: string; label: string };

/** One part that cycles: what it says, and what pressing it moves through. */
export type Cycle = { value: string; label: string; options: readonly Option[] };

/** The sentence over the request, for the Session the words tag. Absent where they tag none. */
export type WaitChip = {
  on: { id: string; title: string };
  when: Cycle;
  base: Cycle;
  /** Absent where the brief asks for nothing. */
  needs?: Cycle;
  /** The parts set by hand. */
  his: readonly WaitPart[];
  set: (part: WaitPart, value: string) => void;
};

/** A condition by the key the sentence cycles it by: the three plain ones, and `plan:<id>` for a part of the plan. */
export const whenKey = (when: StartWhen, part?: { id: string }): string => (when === "plan" && part !== undefined ? `plan:${part.id}` : when);

export function conditionsOf(parts: readonly PlanPart[]): Option[] {
  return [
    { value: "ends", label: "ends" },
    ...parts.map((one) => ({ value: `plan:${one.id}`, label: `${one.title} is done` })),
    { value: "pushed", label: "PR is pushed" },
    { value: "lands", label: "PR lands" },
  ];
}

export const BASES: readonly Option[] = [
  { value: "main", label: "main" },
  { value: "branch", label: "its PR branch" },
];

/** What is asked of the Session: all of it, and each thing when it asks for several. */
export function needsOf(needs: string): Option[] {
  const each = needs.split(/\s+and\s+/i).filter((one) => one !== "");
  return [needs, ...(each.length > 1 ? each : [])].map((one) => ({ value: one, label: one }));
}

/** What a brief names once a hand has been over it. */
export type Chosen = { on: { id: string; title: string }; when: StartWhen; part?: { id: string; title: string }; base: StartBase; needs: string; his: WaitPart[] };

export function chosen(read: WaitReading, hand: Hand, parts: readonly PlanPart[]): Chosen {
  const his: WaitPart[] = [];
  let when = read.when;
  let part = read.part;
  const key = whenKey(read.when, read.part);
  if (hand.when !== undefined && hand.when.was === key) {
    his.push("when");
    const picked = parts.find((one) => `plan:${one.id}` === hand.when!.key);
    when = picked !== undefined ? "plan" : (hand.when.key as StartWhen);
    part = picked === undefined ? undefined : { id: picked.id, title: picked.title };
  }
  let base = read.base;
  if (hand.base !== undefined && hand.base.was === read.base) {
    his.push("base");
    base = hand.base.value;
  }
  let needs = read.needs;
  if (hand.needs !== undefined && hand.needs.was === read.needs) {
    his.push("needs");
    needs = hand.needs.value;
  }
  if (hand.on !== undefined && hand.on.was === read.on.id) his.push("on");
  return { on: read.on, when, ...(part === undefined ? {} : { part }), base, needs, his };
}

/** The hand after `part` is set to `value` on a sentence read as `read`. */
export function handled(read: WaitReading, hand: Hand, part: WaitPart, value: string): Hand {
  if (part === "when") return { ...hand, when: { key: value, was: whenKey(read.when, read.part) } };
  if (part === "base") return { ...hand, base: { value: value as StartBase, was: read.base } };
  if (part === "needs") return { ...hand, needs: { value, was: read.needs } };
  return { ...hand, on: { id: value, was: value } };
}

/** The sentence, from the reading and the hand over it. */
export function chipOf(read: WaitReading, hand: Hand, parts: readonly PlanPart[], set: WaitChip["set"]): WaitChip {
  const one = chosen(read, hand, parts);
  const conditions = conditionsOf(parts);
  const key = whenKey(one.when, one.part);
  const needs = needsOf(one.needs);
  return {
    on: one.on,
    when: { value: key, label: conditions.find((it) => it.value === key)?.label ?? "ends", options: conditions },
    base: { value: one.base, label: BASES.find((it) => it.value === one.base)!.label, options: BASES },
    ...(one.needs === "" ? {} : { needs: { value: one.needs, label: one.needs, options: needs } }),
    his: one.his,
    set,
  };
}

/** Where the tag of the Session called `title` stands in the words: from its `@` to the end of its title. */
export function tagRange(words: string, title: string): [number, number] | undefined {
  const found = new RegExp(`(^|\\s)@${title.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}(?![\\w])`, "i").exec(words);
  if (found === null) return undefined;
  return [found.index + (found[1]?.length ?? 0), found.index + found[0].length];
}

/** The words with the tag of `from` written as `to`'s. */
export function retagged(words: string, from: string, to: string): string {
  const range = tagRange(words, from);
  return range === undefined ? words : `${words.slice(0, range[0])}@${to}${words.slice(range[1])}`;
}

/** The option `step` places from the one standing, wrapping round. */
export function cycled(options: readonly Option[], value: string, step: 1 | -1): string {
  const at = options.findIndex((one) => one.value === value);
  return options[(Math.max(at, 0) + step + options.length) % options.length]?.value ?? value;
}
