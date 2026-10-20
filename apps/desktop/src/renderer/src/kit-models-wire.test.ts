// Kit › Models on Fleet's answers: the offered models read into rows, and what a switch or a choice saves.

import type { ModelChoices, OfferedModel, Setting, SettingsList } from "@armada/protocol";
import { expect, test } from "vitest";

import { readKitModels, saveForChoice, saveForSwitch } from "./kit-models-wire";

const offer = (name: string, version: string, on = true): OfferedModel => ({
  name,
  display: `${name.charAt(0).toUpperCase()}${name.slice(1)} ${version.replace(/^claude-[a-z]+-/, "").replace(/-/g, ".")}`,
  version,
  description: `${name} says`,
  efforts: ["low", "high"],
  on,
});

const held = (offered: OfferedModel[], over: Partial<ModelChoices> = {}): ModelChoices => ({
  models: offered.filter((one) => one.on).map((one) => one.name),
  default: "sonnet",
  session: "opus",
  offered,
  read: { what: "read" },
  ...over,
});

const setting = (key: string, value: string): Setting => ({
  key,
  group: "Agents",
  section: "Model",
  title: key,
  description: "",
  tip: null,
  kind: { kind: "text" },
  default: value,
  value,
  saved: null,
  applies: "live",
  pending_restart: false,
  overridden_by_env: null,
});

const settings = (values: Record<string, string>): SettingsList => ({
  path: "/settings.json",
  refused: null,
  settings: Object.entries(values).map(([key, value]) => setting(key, value)),
});

const FOUR = [offer("opus", "claude-opus-5-5"), offer("fable", "claude-fable-5-1"), offer("sonnet", "claude-sonnet-5-5"), offer("haiku", "claude-haiku-5-5")];
const NAMING = { "models.review": "opus", "models.judge": "haiku", "models.proposer": "haiku", "models.retro": "sonnet", "effort.default": "high" };

test("the offered models are the rows, named without their version", () => {
  const read = readKitModels(held(FOUR), settings(NAMING));
  expect(read).toMatchObject({ read: "read" });
  if (read.read !== "read") return;
  expect(read.models.map((one) => [one.name, one.display, one.resolved, one.on])).toEqual([
    ["opus", "Opus", "claude-opus-5-5", true],
    ["fable", "Fable", "claude-fable-5-1", true],
    ["sonnet", "Sonnet", "claude-sonnet-5-5", true],
    ["haiku", "Haiku", "claude-haiku-5-5", true],
  ]);
  expect(read.choices).toMatchObject({ jobs: "opus", drones: "sonnet", reviews: "opus", effort: "high", judge: "haiku", proposer: "haiku", retro: "sonnet" });
});

test("models Fleet could not read say why", () => {
  const why = "the agent did not answer in time";
  expect(readKitModels(held([offer("opus", "claude-opus-5-5")], { read: { what: "not_read", why } }), null)).toEqual({ read: "not_read", why });
  expect(readKitModels(null, null)).toMatchObject({ read: "not_read" });
});

test("switching a model off saves the roster and moves every setting that names it", () => {
  const all = settings({ ...NAMING, "models.session": "opus", "models.default": "opus", "models.secondOpinion": "sonnet" });
  const changes = saveForSwitch(held(FOUR, { session: "opus", default: "opus" }), all, "opus", false);
  expect(changes).toEqual({
    "models.roster": ["fable", "sonnet", "haiku"],
    "models.session": "fable",
    "models.default": "fable",
    "models.review": "fable",
  });
  const cheap = saveForSwitch(held(FOUR), all, "haiku", false);
  expect(cheap).toEqual({ "models.roster": ["opus", "fable", "sonnet"], "models.judge": "sonnet", "models.proposer": "sonnet" });
});

test("switching a model on saves the roster alone, and the last one on cannot go", () => {
  const off = [offer("opus", "claude-opus-5-5", true), offer("haiku", "claude-haiku-5-5", false)];
  expect(saveForSwitch(held(off), settings({ "models.review": "opus" }), "haiku", true)).toEqual({ "models.roster": ["opus", "haiku"] });
  expect(saveForSwitch(held(off), null, "opus", false)).toBeNull();
});

test("a choice writes the setting its kind of run reads", () => {
  expect(saveForChoice("jobs", "fable")).toEqual({ "models.session": "fable" });
  expect(saveForChoice("drones", "fable")).toEqual({ "models.default": "fable" });
  expect(saveForChoice("reviews", "fable")).toEqual({ "models.review": "fable" });
  expect(saveForChoice("effort", "low")).toEqual({ "effort.default": "low" });
});
