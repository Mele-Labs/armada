// The Cockpit canvas map, for the owner to look at. Over the `pixi-map` scenario, where one Session writes, one reads and one has a red Check.

import { inside, region, role, walk } from "../walk";

const map = role("listbox", "Map");
const drone = (title: RegExp) => inside(map, role("option", title));

const pixiMap = walk("pixi-map", [
  { look: map, say: "The whole field. Each repository has its bays, its plots and a flag for main. A drone hovers over the plots it works in" },
  { hover: drone(/^Fix the reconnect wait/), say: "A Job with a red Check. Its drone shows the failure, and its card names it" },
  { hover: drone(/^Pin the store clock/), say: "A Job writing. Its drone hovers over the pill of the file it is writing, and its card says it is writing" },
  { look: map, say: "Files lie on their plots as pills, lit by what is done to them: green for a file made, blue for one changed. A file two Jobs name shows the stronger act" },
  { hover: drone(/^Migration notes/), say: "A Job reading. Its card names the state and when it last did anything" },
  { press: drone(/^Pin the store clock/), say: "A press on a drone opens its Job in the stage" },
  { look: region("Field"), say: "The Job's Field, with its drone over the file it is writing and the files it names as pills" },
]);

export { pixiMap as "pixi-map" };
