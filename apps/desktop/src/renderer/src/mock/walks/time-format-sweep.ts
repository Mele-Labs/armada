// The one time format on another screen: Studios lists when each was last touched, where an earlier day keeps its day. Over `time-format-sweep`.

import { button, toSystemPage, walk } from "../walk";

const touched = button(/^[A-Z][a-z]{2} \d{1,2} \d/);

export const timeFormatSweep = walk("time-format-sweep", [
  ...toSystemPage("Studios", "Studios"),
  { look: touched, say: "Last touched: a Studio touched on an earlier day carries its day" },
  { press: touched, say: "Pressed, every Last touched reads as an age" },
]);
