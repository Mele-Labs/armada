// The Reviews page keeps its panels' bands: the Cockpit's density pass had floated the list's band over its picker (26 Oct 2026).
// Over the `pr-review` scenario.

import { place, region, walk } from "../walk";

const reviewsBand = walk("pr-review", [
  { press: place("Reviews"), say: "Reviews" },
  { look: region("Pull requests"), say: "The list's band is above the list, clear of its picker" },
  { look: region("Flight controls"), say: "The tools keep their band too" },
]);

export { reviewsBand as "reviews-band" };
