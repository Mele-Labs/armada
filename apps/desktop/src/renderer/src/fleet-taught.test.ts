// The Mods page's words, Fleet, tweak and tracker rows, over a fake Fleet: the list as `mods.changed` carries it, this machine's switch
// for each (shown at once, and taken back with Fleet's reason where it refuses), and Promote.

import { describe, expect, it } from "vitest";
import type { ModSummary, Outcome } from "@armada/protocol";

import { createFleetTaught, NO_TAUGHT } from "./fleet-taught";
import type { FleetTaught } from "./fleet-taught";

const row = (name: string, over: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "words", enabled: true, valid: true, ...over });
const settled = () => new Promise((done) => setTimeout(done, 0));

function fake(mods: ModSummary[] | null, over: Partial<FleetTaught> = {}) {
  const told: string[] = [];
  const switched: string[] = [];
  let hear: Parameters<FleetTaught["subscribe"]>[0] = () => undefined;
  const fleet: FleetTaught = {
    state: async () => ({ mods: mods === null ? null : { mods } }),
    subscribe: (on) => ((hear = on), () => undefined),
    setModEnabled: async (name, enabled) => (switched.push(`${enabled ? "on" : "off"} ${name}`), { ok: true }),
    promoteMod: async (name) => ({ ok: true, modPromoted: { name, branch: `armada/mod-${name}-1`, commit: "abc" } }),
    validateMod: async () => null,
    ...over,
  };
  const source = createFleetTaught(fleet, (sentence) => told.push(sentence));
  return { source, told, switched, push: (next: ModSummary[]) => hear({ mods: { mods: next } }) };
}

const REFUSED: Outcome = { ok: false, why: "not_connected" };

describe("the rows", () => {
  it("are the Words, Fleet, tweak and tracker mods, each with what it does and what it was taught on, and no theme or layout", async () => {
    const { source } = fake([
      row("jira-keys", { description: "Jira keys link to each repository's site", taught_on: "STORE-1234" }),
      row("batch", { kind: "fleet", description: "Slot behind main notices batched", taught_on: "the notice slot-14 is 25 commits behind main" }),
      row("louder", { kind: "tweak", enabled: false }),
      row("linear", { kind: "tracker" }),
      row("dusk", { kind: "theme" }),
      row("tidy", { kind: "layout" }),
    ]);
    await settled();
    expect(source.get().mods.map((one) => [one.name, one.kind, one.enabled])).toEqual([
      ["jira-keys", "words", true],
      ["batch", "fleet", true],
      ["louder", "tweak", false],
      ["linear", "tracker", true],
    ]);
    expect(source.get().mods[0]).toMatchObject({ description: "Jira keys link to each repository's site", on: "STORE-1234" });
  });

  it("carry Fleet's reason where a mod did not pass its checks", async () => {
    const { source } = fake([row("bad", { valid: false, reason: "line 3: not a pattern" }), row("worse", { valid: false })]);
    await settled();
    expect(source.get().mods.map((one) => one.description)).toEqual(["line 3: not a pattern", "Did not pass its checks"]);
  });

  it("are replaced whole by each list Fleet sends", async () => {
    const { source, push } = fake([row("one")]);
    await settled();
    push([row("two")]);
    expect(source.get().mods.map((one) => one.name)).toEqual(["two"]);
  });

  it("keep what a layout mod was taught on, for its row on the Layout page to say", async () => {
    const { source } = fake([row("hide-field", { kind: "layout", taught_on: "the Field panel" }), row("tidy", { kind: "layout" })]);
    await settled();
    expect(source.get().details).toEqual({ "hide-field": { on: "the Field panel", parts: [] } });
  });

  it("open to what Fleet's checked file holds, read for each mod once and again when the mod changes", async () => {
    const reads: string[] = [];
    const validateMod = async (name: string) => (reads.push(name), { name, valid: true, problems: [], words: { name: "Jira key", pattern: "[A-Z]+-\\d+", example: "STORE-1234", address: [{ repository: "armada", template: "https://acme.atlassian.net/browse/{match}", found: "settings" }] } });
    const { source, push } = fake([row("jira-keys", { changed_at: "1" }), row("hide-field", { kind: "layout", taught_on: "the Field panel", changed_at: "1" }), row("linear", { kind: "tracker" }), row("dusk", { kind: "theme" })], { validateMod });
    await settled();
    expect(reads).toEqual(["jira-keys", "hide-field"]);
    expect(source.get().mods[0]?.parts).toEqual([
      { part: "Words", facts: [{ label: "Pattern", value: "[A-Z]+-\\d+", code: true }, { label: "armada", value: "https://acme.atlassian.net/browse/{match}", code: true, from: "Found in this repository's settings" }] },
    ]);
    push([row("jira-keys", { changed_at: "1" })]);
    await settled();
    expect(reads).toEqual(["jira-keys", "hide-field"]);
    push([row("jira-keys", { changed_at: "2" })]);
    await settled();
    expect(reads).toEqual(["jira-keys", "hide-field", "jira-keys"]);
  });

  it("say a Fleet mod's rules in the words its row opened uses, not the file's own description", async () => {
    const validateMod = async (name: string) => ({ name, valid: true, problems: [], fleet: { rule: [{ notice: "behind_base", does: "batch" as const, where: "repo armada", when: [{ if: "blocks_merge" as const, does: "tell" as const }] }] } });
    const { source } = fake([row("slot-behind", { kind: "fleet", description: "behind_base: batch repo armada" })], { validateMod });
    await settled();
    expect(source.get().mods[0]?.description).toMatch(/^Slot behind notice, batched .*told only when it blocks a merge$/);
  });

  it("have no parts where Fleet has no file for the mod", async () => {
    const { source } = fake([row("jira-keys")]);
    await settled();
    expect(source.get().mods[0]?.parts).toEqual([]);
  });
});

describe("the switch", () => {
  it("is shown as pressed at once and sent to Fleet", async () => {
    const { source, switched } = fake([row("jira-keys")]);
    await settled();
    source.setEnabled("jira-keys", false);
    expect(source.get().mods[0]?.enabled).toBe(false);
    await settled();
    expect(switched).toEqual(["off jira-keys"]);
  });

  it("is taken back, with Fleet's words, where Fleet refuses it", async () => {
    const { source, told } = fake([row("jira-keys")], { setModEnabled: async () => REFUSED });
    await settled();
    source.setEnabled("jira-keys", false);
    await settled();
    expect(source.get().mods[0]?.enabled).toBe(true);
    expect(told).toHaveLength(1);
  });
});

describe("Promote", () => {
  it("names the branch Fleet cut, and says so where Fleet refuses", async () => {
    const kept = fake([row("jira-keys")]);
    await settled();
    kept.source.promote("jira-keys");
    await settled();
    expect(kept.source.get().mods[0]?.branch).toBe("armada/mod-jira-keys-1");
    const refused = fake([row("jira-keys")], { promoteMod: async () => REFUSED });
    await settled();
    refused.source.promote("jira-keys");
    await settled();
    expect(refused.source.get().mods[0]).not.toHaveProperty("branch");
    expect(refused.told).toHaveLength(1);
  });
});

it("lists nothing in safe mode, which reads no mod", () => {
  expect(NO_TAUGHT.get()).toEqual({ mods: [], details: {} });
  // A snapshot read twice is the same object, or React draws again for ever.
  expect(NO_TAUGHT.get()).toBe(NO_TAUGHT.get());
});
