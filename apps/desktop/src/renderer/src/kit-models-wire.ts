// Kit › Models on Fleet's `list_models` and settings.json: the offered models drawn as the page's rows, and
// the saves that a switch or a choice makes. Pure, so the page's hook only sends what this builds.

import type { KitModel, KitModelChoices, KitModelSlot, KitModelsRead } from "@armada/components";
import type { ModelChoices, OfferedModel, SaveSettings, SettingsList } from "@armada/protocol";

/** The setting each choice on the page writes. */
const CHOICE_KEY: Readonly<Record<KitModelSlot, string>> = {
  jobs: "models.session",
  drones: "models.default",
  reviews: "models.review",
  effort: "effort.default",
  judge: "models.judge",
  judgeEffort: "effort.judge",
  proposer: "models.proposer",
  proposerEffort: "effort.proposer",
  retro: "models.retro",
  secondOpinion: "models.secondOpinion",
  downshift: "plan.downshiftTo",
  helmEffort: "effort.helm",
};

/**
 * Every setting that names a model, and which model it falls back to when the one it names is switched
 * off. Fleet refuses a save that leaves one naming an off model, so the same save moves them. The
 * cheap tier falls to the last model on (the roster runs strongest first), the rest to the first.
 */
const FALLS_TO: Readonly<Record<string, "first" | "last">> = {
  "models.session": "first",
  "models.default": "first",
  "models.review": "first",
  "models.retro": "first",
  "models.secondOpinion": "first",
  "plan.downshiftTo": "first",
  "models.judge": "last",
  "models.proposer": "last",
  "models.account": "last",
};

const valueOf = (settings: SettingsList | null, key: string): string | undefined => {
  const value = settings?.settings.find((one) => one.key === key)?.value;
  return typeof value === "string" ? value : undefined;
};

/** `Opus 5.5` is `Opus`: the page draws the version beside it. */
const familyOf = (offered: OfferedModel): string =>
  (offered.display ?? "").replace(/\s+\d+(\.\d+)*$/, "") || offered.name.charAt(0).toUpperCase() + offered.name.slice(1);

export const NOT_ANSWERED: KitModelsRead = { read: "not_read", why: "Fleet did not answer" };

/** What the page draws: the offered models, and the model each kind of run starts on. */
export function readKitModels(held: ModelChoices | null, settings: SettingsList | null): KitModelsRead {
  if (held === null) return NOT_ANSWERED;
  if (held.read?.what === "not_read") return { read: "not_read", why: held.read.why };
  if (held.read === undefined || held.offered === undefined) return { read: "not_read", why: "Fleet does not list models yet" };
  const models: KitModel[] = held.offered.map((one) => ({
    name: one.name,
    display: familyOf(one),
    resolved: one.version ?? "",
    says: one.description ?? "",
    effort_levels: one.efforts ?? [],
    on: one.on,
  }));
  const model = (slot: KitModelSlot) => valueOf(settings, CHOICE_KEY[slot]) ?? held.default;
  const effort = (slot: KitModelSlot) => valueOf(settings, CHOICE_KEY[slot]) ?? "harness default";
  const choices: KitModelChoices = {
    jobs: held.session,
    drones: held.default,
    reviews: model("reviews"),
    effort: effort("effort"),
    judge: model("judge"),
    judgeEffort: effort("judgeEffort"),
    proposer: model("proposer"),
    proposerEffort: effort("proposerEffort"),
    retro: model("retro"),
    secondOpinion: model("secondOpinion"),
    downshift: model("downshift"),
    helmEffort: effort("helmEffort"),
  };
  return { read: "read", models, choices };
}

/**
 * The save for switching one model on or off: the roster, and every setting that names the model being
 * switched off, moved to one still on. `null` when it would leave none on.
 */
export function saveForSwitch(held: ModelChoices, settings: SettingsList | null, name: string, on: boolean): SaveSettings["changes"] | null {
  const roster = (held.offered ?? []).filter((one) => (one.name === name ? on : one.on)).map((one) => one.name);
  const first = roster[0];
  const last = roster[roster.length - 1];
  if (first === undefined || last === undefined) return null;
  const changes: SaveSettings["changes"] = { "models.roster": roster };
  for (const [key, falls] of Object.entries(FALLS_TO)) {
    const named = valueOf(settings, key);
    if (named !== undefined && !roster.includes(named)) changes[key] = falls === "first" ? first : last;
  }
  return changes;
}

/** The save for choosing a model for a kind of run, or the effort. */
export const saveForChoice = (slot: KitModelSlot, value: string): SaveSettings["changes"] => ({ [CHOICE_KEY[slot]]: value });
