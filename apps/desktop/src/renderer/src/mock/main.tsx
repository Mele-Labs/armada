// The mock's page: the app on `?scenario=<name>`, and the picker beside it.

import type { WorkflowStepCardDesign } from "@armada/components";

import { mountApp } from "./mount";
import { CARD_DESIGNS, mountPicker } from "./Picker";
import { SCENARIOS, scenarioNamed } from "./scenario";

const query = new URLSearchParams(window.location.search);
const asked = query.get("scenario");
// `?frame` draws the app alone, for Evidence to photograph: no picker over it.
const framing = query.has("frame");
// `?cards=progress|needs|compact` draws the Workflow tab's step cards in one of
// the three designs the owner is comparing (30 Sep 2026). Absent is today's.
const askedCards = query.get("cards");
const cards = CARD_DESIGNS.find((one) => one === askedCards) as WorkflowStepCardDesign | undefined;
const scenario = (asked === null ? undefined : scenarioNamed(asked)) ?? SCENARIOS[0]!;
if (asked !== null && asked !== scenario.name) {
  console.warn(`no mock scenario named ${asked}; showing ${scenario.name}`);
}

const root = document.getElementById("root");
const picker = document.getElementById("picker");
if (root !== null && picker !== null) {
  mountApp(scenario, root, undefined, cards);
  if (!framing) mountPicker(scenario.name, picker, cards);
}

// The annotation layer (#1226), saving through this dev server's `annotationsServer`. Not in a frame.
if (!framing) void import("../annotate/mount").then(({ mount }) => mount());
