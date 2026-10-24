// Mod Jobs fall in the Mods band, whichever repository is picked and however many are served; a list with none is drawn as it was.

import { expect, it } from "vitest";
import type { RepositorySummary } from "@armada/protocol";

import { bandingOf, bandsOf, MODS_BAND, OTHER_BAND } from "./bands";
import type { Subject } from "./ship";

const repository = (id: string): RepositorySummary => ({ root: `/code/${id}`, manifest: { id } }) as unknown as RepositorySummary;
const job = (id: string, manifest: string | undefined, mod = false): Subject =>
  ({ key: `session:${id}`, kind: "session", id, ...(manifest === undefined ? {} : { manifest }), ...(mod ? { mod: true } : {}) }) as unknown as Subject;

const armada = repository("armada");
const billing = repository("billing");

/** The labels of the bands the list draws, and the keys in each. */
const drawn = (subjects: Subject[], picked: RepositorySummary | null, served: RepositorySummary[]) => {
  const banding = bandingOf(picked, served, subjects.some((one) => one.mod === true));
  return banding === undefined ? undefined : bandsOf(subjects, banding).map((band) => [band.label, band.subjects.map((one) => one.key)]);
};

it.each([
  ["no repository picked, two served", null, [armada, billing]],
  ["no repository picked, one served", null, [armada]],
  ["no repository picked, none served", null, []],
  ["one repository picked, two served", billing, [armada, billing]],
  ["one repository picked, one served", armada, [armada]],
])("puts a Mod Job under Mods with %s", (_name, picked, served) => {
  const bands = drawn([job("mod", "armada", true), job("plain", "armada")], picked, served);
  expect(bands).toBeDefined();
  expect(bands!.find(([label]) => label === MODS_BAND)).toEqual([MODS_BAND, ["session:mod"]]);
  expect(bands!.flatMap(([, keys]) => keys as string[]).filter((key) => key === "session:mod")).toHaveLength(1);
});

it("keeps a Mod Job out of its repository's band and puts Mods after the repositories and before Other", () => {
  const bands = drawn([job("mod", "armada", true), job("a", "armada"), job("b", "billing"), job("loose", undefined)], null, [armada, billing]);
  expect(bands).toEqual([
    ["armada", ["session:a"]],
    ["billing", ["session:b"]],
    [MODS_BAND, ["session:mod"]],
    [OTHER_BAND, ["session:loose"]],
  ]);
});

it("draws the list as it was where no Job is a Mod Job and one repository is picked or served", () => {
  expect(drawn([job("a", "armada")], null, [armada])).toBeUndefined();
  expect(drawn([job("a", "armada")], armada, [armada, billing])).toBeUndefined();
});

it("groups by repository on All with several served, with no Mod Job", () => {
  expect(drawn([job("a", "armada"), job("b", "billing")], null, [armada, billing])).toEqual([
    ["armada", ["session:a"]],
    ["billing", ["session:b"]],
  ]);
});
