// The scenario list: every row has a name of its own, every walk plays on a scenario that exists, and
// the Studios, merge lines and rows listed from `scenarios/` all reach the list.

import { expect, test } from "vitest";

import { SCENARIOS, scenarioNamed } from "./scenario";
import { WALKS } from "./walks";

const ROW_FILES = import.meta.glob<Record<string, unknown>>("./scenarios/*.ts", { eager: true });

test("every scenario has its own name", () => {
  const names = SCENARIOS.map((one) => one.name);
  expect(new Set(names).size).toBe(names.length);
});

// A walk over a name no scenario has would be played against a default, so it is refused here.
test("every walk plays on a scenario the list holds", () => {
  const names = new Set(SCENARIOS.map((one) => one.name));
  for (const [walk, one] of WALKS) expect(names, `walk ${walk} plays on ${one.scenario}`).toContain(one.scenario);
});

// A Studio on a Manifest the scenario does not serve is never read, so the surface would draw an
// empty list and nothing would say why — #1341.
test("every Studio a scenario keeps belongs to a Manifest that scenario serves", () => {
  for (const scenario of SCENARIOS) {
    const served = (scenario.state.holds.manifests ?? []).map((one) => one.id);
    for (const studio of scenario.studios ?? []) {
      expect(served, `${scenario.name} keeps a Studio on a Manifest it does not serve`).toContain(
        studio.manifest_id,
      );
    }
  }
});

// Studios' scenarios are built in `@armada/studios/fake` and listed through its slice.
test("the Studios scenarios resolve by name", () => {
  for (const name of ["studios", "studio-zone", "studio-zone-proposal", "studio-read-nothing"]) {
    expect(scenarioNamed(name)?.name, name).toBe(name);
  }
});

test("the mock opens on a scenario that serves a Studio", () => {
  const first = SCENARIOS[0]!;
  expect(first.name).toBe("default");
  expect(first.studios?.length).toBeGreaterThan(0);
});

test("the Lessons and Cleanup scenarios close the list", () => {
  expect(SCENARIOS.slice(-3).map((one) => one.name)).toEqual(["retro/lessons", "cleanup/slots", "cleanup/grid"]);
});

test("no scenario names a shape", () => {
  for (const one of SCENARIOS) {
    const said = `${one.name} ${one.says}`.toLowerCase();
    for (const word of ["convoy", "train", "atomic"]) {
      expect(new RegExp(`\\b${word}\\b`).test(said), `${one.name}: ${one.says}`).toBe(false);
    }
  }
});

// A name two files both export would overwrite one with the other, which would drop a walk's row
// without saying so.
test("every row exported from scenarios/ has a name of its own, and reaches the list", () => {
  const exported = Object.values(ROW_FILES).flatMap((one) => Object.keys(one));
  expect(new Set(exported).size).toBe(exported.length);
  const names = SCENARIOS.map((one) => one.name);
  for (const file of Object.values(ROW_FILES)) {
    for (const row of Object.values(file)) expect(names).toContain((row as { name: string }).name);
  }
});
