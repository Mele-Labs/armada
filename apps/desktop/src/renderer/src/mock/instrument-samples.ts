// Mock-only: three instruments as mods would write them. Reviews draws, Spend throws, and Main build is what a Mod Job hands back from Make one.

import { createElement as h } from "react";
import { CircleDollarSign, GitPullRequest, Hammer } from "lucide-react";

import { buildingFace, type FacePaint, type InstrumentMod } from "../cockpit/deck/instrument-mods";

const WAITING = [
  { number: 3171, title: "Seat a mod's instrument in a bay", age: 22 },
  { number: 3168, title: "Keep the rack's tray to one row", age: 9 },
  { number: 3164, title: "Fold the merge queue with the console", age: 5 },
  { number: 3160, title: "Name the Calls HUD by its Job", age: 2 },
  { number: 3158, title: "Draw hi-fi sketches with the repo's stylesheets", age: 1 },
];
const RUNS = [38, 41, 36, 52, 44, 39, 61, 47, 42, 40, 35, 37, 58, 43, 41, 36];

const hue = (age: number, ink: { bad: string; ask: string; run: string }) => (age >= 12 ? ink.bad : age >= 4 ? ink.ask : ink.run);

const paintReviews: FacePaint = (draw, { w, h: tall, ink }) => {
  const lamp = Math.min(tall * 0.34, (w - 16) / WAITING.length - 4);
  WAITING.forEach((one, at) => {
    draw.fillStyle = hue(one.age, ink);
    draw.fillRect(8 + at * (lamp + 4), 8, lamp, lamp);
  });
  draw.font = `600 ${Math.round(tall * 0.2)}px ui-monospace, Menlo, monospace`;
  draw.textBaseline = "alphabetic";
  draw.fillStyle = ink.fg;
  draw.fillText(`#${WAITING[0]!.number}`, 8, tall - 8);
  const age = `${WAITING[0]!.age}h`;
  draw.fillStyle = hue(WAITING[0]!.age, ink);
  draw.fillText(age, w - 8 - draw.measureText(age).width, tall - 8);
};

const paintRuns: FacePaint = (draw, { w, h: tall, t, ink }) => {
  const top = Math.max(...RUNS);
  const each = (w - 16) / RUNS.length;
  const moving = Math.floor(t * 2) % RUNS.length;
  RUNS.forEach((one, at) => {
    const bar = ((tall - 16) * one) / top;
    draw.fillStyle = at === moving ? ink.fg : one > 50 ? ink.ask : ink.run;
    draw.fillRect(8 + at * each, tall - 8 - bar, Math.max(1, each - 2), bar);
  });
};

function ReviewsPanel() {
  return h(
    "ul",
    { className: "instrument-sample__rows" },
    WAITING.map((one) => h("li", { key: one.number }, h("i", { "data-age": one.age >= 12 ? "old" : one.age >= 4 ? "due" : "new" }), h("code", null, `#${one.number}`), h("span", null, one.title), h("time", null, `${one.age}h`))),
  );
}

function RunsPanel() {
  return h(
    "ul",
    { className: "instrument-sample__rows" },
    RUNS.slice(-6).map((one, at) => h("li", { key: at }, h("i", { "data-age": one > 50 ? "due" : "new" }), h("code", null, `${one}m`), h("span", null, `ci on main, run ${at + 1}`))),
  );
}

export const REVIEWS: InstrumentMod = { name: "reviews", label: "Reviews", code: "REV", Icon: GitPullRequest, paint: paintReviews, Panel: ReviewsPanel };
export const SPEND: InstrumentMod = {
  name: "spend",
  label: "Spend",
  code: "SPD",
  Icon: CircleDollarSign,
  paint: () => {
    throw new Error("usage.month is undefined");
  },
};
export const MAIN_BUILD: InstrumentMod = { name: "main-build", label: "Main build", code: "CI", Icon: Hammer, paint: paintRuns, Panel: RunsPanel };
export const MAIN_BUILD_BUILDING: InstrumentMod = { name: "main-build", label: "Main build", code: "CI", Icon: Hammer, paint: buildingFace };
