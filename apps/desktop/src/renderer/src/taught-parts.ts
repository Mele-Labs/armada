// What a taught mod holds, read from the file Fleet checked (`validate_mod`): the pattern and addresses of a Words mod, the rules of a
// Fleet mod, the tokens a tweak sets, what a layout hides. It is what a mod's row opens to, and what a Logbook entry's `Also changes`
// line words. Nothing here is kept by the window: Fleet's file is the record.

import type { FleetFile, FleetRule, ModChecked, WordsFile } from "@armada/protocol";
import type { TaughtPart } from "@armada/settings";

import { DOES as ALSO_DOES, SCOPE, foundSaid } from "./teach";
import { DOES, layoutOf, noticeName, scopeOf } from "./teach-wire";

/** The repository a Fleet rule is for, from the place Fleet wrote: `repo <manifest>`. */
const repositoryOf = (where: string): string => (where.startsWith("repo ") ? where.slice("repo ".length) : "");

const wordsPart = ({ pattern, address }: WordsFile): TaughtPart => ({
  part: "Words",
  facts: [
    { label: "Pattern", value: pattern, code: true },
    ...(address ?? []).map((one) => ({ label: one.repository === "*" ? "Other repositories" : one.repository, value: one.template, code: true, from: foundSaid(one.found) })),
  ],
});

const fleetPart = (rule: FleetRule): TaughtPart => ({
  part: "Fleet",
  facts: [
    { label: "Notice", value: noticeName(rule.notice) },
    { label: "Does", value: DOES[rule.does] },
    { label: "Where", value: scopeOf(rule.where, repositoryOf(rule.where)) },
  ],
});

/** The tokens a tweak's stylesheet sets: `:root { --token: value; }`. */
function tweakPart(css: string): TaughtPart {
  const sets = [...css.matchAll(/(--[\w-]+)\s*:\s*([^;}]+)/g)];
  return {
    part: "Theme tweak",
    facts: sets.flatMap(([, token, now]) => [
      { label: "Token", value: token ?? "", code: true },
      { label: "Now", value: (now ?? "").trim(), code: true },
    ]),
  };
}

/** What a checked mod holds, as the parts its row reads. A mod that did not pass holds nothing here. */
export function partsOf(checked: ModChecked): readonly TaughtPart[] {
  if (!checked.valid) return [];
  return [
    ...(checked.words === undefined ? [] : [wordsPart(checked.words)]),
    ...(checked.fleet?.rule ?? []).map(fleetPart),
    ...(checked.css === undefined ? [] : [tweakPart(checked.css)]),
    ...(checked.layout === undefined ? [] : [layoutPart(checked.name, checked.layout)]),
  ];
}

function layoutPart(name: string, text: string): TaughtPart {
  const { hides, where } = layoutOf(name, text);
  return { part: "Layout", facts: [{ label: "Hides", value: hides }, { label: "In", value: where }] };
}

const lower = (text: string): string => `${text.charAt(0).toLowerCase()}${text.slice(1)}`;

/** A Fleet file's rules in one line: the notice, what is done with it and where. */
export function fleetAlso({ rule }: FleetFile): string | undefined {
  const said = (rule ?? []).map((one) => {
    const reach = one.where === "everywhere" ? SCOPE.all : one.where.startsWith("job ") ? SCOPE.job : SCOPE.repository;
    return `${lower(noticeName(one.notice))}, ${ALSO_DOES[one.does]} ${reach}`;
  });
  return said.length === 0 ? undefined : said.join("; ");
}
