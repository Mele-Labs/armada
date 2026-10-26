// The mock's page: the app on `?scenario=<name>`, or on `?walk=<name>` the
// walk's own scenario with the walk played over it — and the picker, on a
// root of its own whose own host stays empty because what it draws goes into the app's
// left column through a portal.

import { forgetAllKept } from "@armada/components";

import { forgetDismissals } from "../cockpit/dismissed";
import { forgetViewed } from "../cockpit/viewed";
import { annotatesWith } from "./annotating";
import { inlineWalk } from "./inline-walk";
import { mountApp } from "./mount";
import { mountPicker } from "./Picker";
import { forgetHowItWasRead, meetEveryGuide } from "./remembered";
import { mountRunner } from "./review-walk-runner";
import type { RunnerPage } from "./review-walk-runner";
import { SCENARIOS, scenarioNamed } from "./scenario";
import { mountNoWalk, mountWalk } from "./WalkPlayer";
import { EVERY_WALK } from "./walks";

const query = new URLSearchParams(window.location.search);
// `?switch=rail` is the take of the switch the walk is shown in; with none the track is a cut box.
const switchTake = query.get("switch");
if (switchTake !== null) document.documentElement.dataset.switch = switchTake;
// A walk names its scenario, so `?walk` wins over `?scenario`.
// `?steps=` is a walk handed over whole, as `show_window` hands Bridge its steps.
const inline = inlineWalk(query.get("steps"), query.get("scenario") ?? "every-state");
const walking = query.get("walk") ?? (inline === undefined ? null : "A walk of the change");
const script = inline ?? (walking === null ? undefined : EVERY_WALK.get(walking));
const asked = script?.scenario ?? query.get("scenario");
// `?frame` draws the app alone, for Evidence to photograph: no picker over it.
const framing = query.has("frame");
const scenario = (asked === null ? undefined : scenarioNamed(asked)) ?? SCENARIOS[0]!;
if (asked !== null && asked !== scenario.name) {
  console.warn(`no mock scenario named ${asked}; showing ${scenario.name}`);
}

// A walk starts from the window its test starts from, so what it shows is what CI ran. A replay in the
// same browser must not inherit the Sessions the last one viewed, the calls it dismissed or the drafts it left.
if (script !== undefined) {
  forgetViewed();
  forgetDismissals();
  forgetAllKept();
  forgetHowItWasRead();
  meetEveryGuide();
}

// `?step=<n>` starts a walk on step n, the steps before it run straight through: what Back does.
const startStep = (of: number) => Math.min(Math.max(Number(query.get("step") ?? 1) || 1, 1), of) - 1;

const root = document.getElementById("root");
const picker = document.getElementById("picker");
if (root !== null && picker !== null) {
  // `?runner=` is an API walk's page: the requests it can send, in place of the app.
  const runner = query.get("runner");
  if (runner === null) mountApp(scenario, root);
  else mountRunner(root, JSON.parse(runner) as RunnerPage);
  if (walking !== null && script !== undefined) mountWalk(walking, script, query.has("autoplay"), picker, startStep(script.steps.length));
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
// frame, and not inside a walk window's stand-in (`?walked`), whose own capture takes
// ⌥⌘A there as main's walk window does.
if (!framing && !query.has("walked") && !annotatesWith(scenario.name)) void import("../annotate/mount").then(({ mount }) => mount());
