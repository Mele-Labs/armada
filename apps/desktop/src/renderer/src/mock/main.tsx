// The mock's page: the app on `?scenario=<name>`, or on `?walk=<name>` the
// walk's own scenario with the walk played over it — and the picker, on a
// root of its own whose own host stays empty because what it draws goes into the app's
// left column through a portal.

import { setPlanLayout } from "@armada/screens/src/plan-layout";

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

if (script?.plan !== undefined) setPlanLayout(script.plan);

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
  // A walk page keeps the picker, on a host of its own: the walk's card holds `#picker`, and one
  // element takes one root. Its label names the walk, and `?frame` still has none.
  if (walking !== null && !framing) {
    const beside = document.createElement("div");
    document.body.append(beside);
    mountPicker(walking, beside, script !== undefined);
  }
}

// The annotation layer (#1226), saving through this dev server's `annotationsServer`. Not in a
// frame, and not inside `prototype-walked`'s stand-in window (`?walked`), whose own capture takes
// ⌥⌘A there as main's walk window does.
if (!framing && !query.has("walked")) void import("../annotate/mount").then(({ mount }) => mount());
