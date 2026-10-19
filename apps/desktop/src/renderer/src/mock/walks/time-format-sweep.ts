// The one time format on other screens: the Cockpit's timeline draws ages and a press turns them to clocks, where an
// event from an earlier day keeps its day; Studios lists when each was last touched. Over `time-format-sweep`.

import { button, inside, region, tab, toSystemPage, walk } from "../walk";

const timeline = region("Timeline");
const controls = region("Flight controls");
const age = inside(timeline, button(/ago$/));
const dated = inside(timeline, button(/^[A-Z][a-z]{2} \d{1,2} \d/));
const touched = button(/^[A-Z][a-z]{2} \d{1,2} \d/);

export const timeFormatSweep = walk("time-format-sweep", [
  { press: inside(controls, button("Timeline")), say: "Timeline, from the flight controls" },
  { press: tab("Feed"), say: "Feed" },
  { look: age, say: "Each event carries how long ago it happened" },
  { press: age, say: "Pressed, every age in the feed reads as a clock" },
  { look: dated, say: "The pull request that merged three days ago carries its day. Today's carry the time only" },
  { press: dated, say: "Pressed again, the ages are back" },
  ...toSystemPage("Studios", "Studios"),
  { look: touched, say: "Last touched: a Studio touched on an earlier day carries its day" },
  { press: touched, say: "Pressed, every Last touched reads as an age" },
]);
