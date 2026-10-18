// The Cockpit canvas map with every room full, for the owner to look at: a pill for each file of a directory, quiet until a Session works on
// its file. Plays over the `pixi-map` scenario. A pill is hovered with the pointer, which
// a step cannot do, so the step that names a file says to.

import { inside, role, walk } from "../walk";

const map = role("listbox", "Map");
const drone = (title: RegExp) => inside(map, role("option", title));

const fullRooms = walk("pixi-map", [
  { look: map, say: "The whole field. Each room holds one pill for every file in its directory: a few for a small room, a fine texture for the biggest" },
  { look: map, say: "Hover a pill. Its card names the file with its path from the repository root. A directory names its first hundred files; pills past those give the directory" },
  { hover: drone(/^Pin the store clock/), say: "A Job writing. Its drone hovers over the lit pill of the file it is writing, green for a file made and blue for one changed" },
  { look: map, say: "Zoom into the biggest room with Ctrl and the wheel. The pills keep their places and grow, and each file is always in the same one" },
]);

export { fullRooms as "full-rooms" };
