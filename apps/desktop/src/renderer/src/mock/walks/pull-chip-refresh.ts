// A right press on a pull request chip offers Refresh: Fleet reads it from the forge now, and the chip, its card and the Job's
// thread change by Fleet's events. In the merge line, then in Done. The calls are asserted in `../pull-chip-refresh.test.tsx`.

import { button, deckRow, inside, region, role, text, walk } from "../walk";

const queue = region("Merge queue");
const chip = (state: RegExp) => inside(queue, button(state));
const landingChip = inside(region("Done", { exact: true }), button(/^Pull request #2290 /));

const refresh = walk("pull-chip-refresh", [
  { look: chip(/^#2290 .*ci running/), say: "#2290 in the merge line, its checks running" },
  { context: chip(/^#2290 /), say: "A right press on the chip" },
  { look: role("menu"), say: "Pull request #2290, and Refresh" },
  { press: role("menuitem", "Refresh"), say: "Refresh: Fleet reads it from the forge now" },
  { look: chip(/^#2290 .*ci failed/), say: "The chip reads failed" },
  { hover: chip(/^#2290 /), say: "So does its card" },
  { press: deckRow("Who sees your config"), say: "Open the Job that holds it" },
  { press: inside(region("Fleet turn"), button(/Fleet notice/)), say: "Open Fleet's folded notice" },
  { look: inside(region("Fleet turn"), text(/failed its checks at 4be1c07/)), say: "Fleet told the Job, as the pull reading would" },
  { key: "Alt+1", on: region("Done", { exact: true }), say: "Done is folded until pressed: ⌥1 opens it" },
  { context: landingChip, say: "A right press on the pull request in Done" },
  { look: role("menu"), say: "The same menu" },
  { press: role("menuitem", "Refresh"), say: "Refresh" },
  { look: landingChip, say: "Its checks read failed, and Fleet tells the Job once" },
]);

const openJob = walk("pull-chip-refresh", [
  { hover: chip(/^#2290 /), say: "The chip's card names the Job that holds the pull request" },
  { press: inside(role("tooltip"), button("Open Job Who sees your config")), say: "The Job row is a link" },
  { look: region(/^Job: Who sees your config/), say: "The Job opens" },
]);

export { refresh as "pull-chip-refresh", openJob as "pull-chip-open-job" };
