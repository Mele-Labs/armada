// The mock's page: the app on `?scenario=<name>`, and the picker beside it; or
// on `?walk=<name>`, the walk's own scenario with the walk played over it.

import { mountApp } from "./mount";
import { mountPicker } from "./Picker";
import { forgetHowItWasRead, meetEveryGuide } from "./remembered";
import { SCENARIOS, scenarioNamed } from "./scenario";
import { mountNoWalk, mountWalk } from "./WalkPlayer";
import { EVERY_WALK } from "./walks";

const query = new URLSearchParams(window.location.search);
// A walk names its scenario, so `?walk` wins over `?scenario`.
const walking = query.get("walk");
const script = walking === null ? undefined : EVERY_WALK.get(walking);
const asked = script?.scenario ?? query.get("scenario");
// `?frame` draws the app alone, for Evidence to photograph: no picker over it.
const framing = query.has("frame");
const scenario = (asked === null ? undefined : scenarioNamed(asked)) ?? SCENARIOS[0]!;
if (asked !== null && asked !== scenario.name) {
  console.warn(`no mock scenario named ${asked}; showing ${scenario.name}`);
}

// A walk starts from the window its test starts from, so what it shows is what CI ran.
if (script !== undefined) {
  forgetHowItWasRead();
  meetEveryGuide();
}

const root = document.getElementById("root");
const picker = document.getElementById("picker");
if (root !== null && picker !== null) {
  mountApp(scenario, root);
  if (walking !== null && script !== undefined) mountWalk(walking, script, query.has("autoplay"), picker);
  else if (walking !== null) mountNoWalk(walking, picker);
  else if (!framing) mountPicker(scenario.name, picker);
}

// The annotation layer (#1226), saving through this dev server's `annotationsServer`. Not in a frame.
if (!framing) void import("../annotate/mount").then(({ mount }) => mount());
