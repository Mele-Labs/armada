// The one time format on other screens: the Cockpit's timeline draws ages and a press turns them to clocks; Studios
// lists when each was last touched, where an earlier day keeps its day. Over `time-format-sweep`.

import { button, inside, region, tab, toSystemPage, walk } from "../walk";

const timeline = region("Timeline");
const controls = region("Flight controls");
const age = inside(timeline, button(/ago$/));
const clock = inside(timeline, button(/\d:\d\d/));
const touched = button(/^[A-Z][a-z]{2} \d{1,2} \d/);

export const timeFormatSweep = walk("time-format-sweep", [
  { press: inside(controls, button("Timeline")), say: "Timeline, from the flight controls" },
  { press: tab("Feed"), say: "Feed" },
  { look: age, say: "Each event carries how long ago it happened" },
  { press: age, say: "Pressed, every age in the feed reads as a clock" },
  { look: clock, say: "Today's events carry the time only" },
  { press: clock, say: "Pressed again, the ages are back" },
  ...toSystemPage("Studios", "Studios"),
  { look: touched, say: "Last touched: a Studio touched on an earlier day carries its day" },
  { press: touched, say: "Pressed, every Last touched reads as an age" },
]);
