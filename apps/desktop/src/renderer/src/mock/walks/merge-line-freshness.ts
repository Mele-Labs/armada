// How old the merge queue's reading is, and the forge going quiet. Hovering main's chip adds when Fleet last read the forge to what
// it says. Then GitHub stops answering: main's dot is dimmed and hollow, and an unplugged glyph beside it says since when. Over
// `merge-line-freshness`.

import { inside, region, role, walk, type Step } from "../walk";

const queue = region("Merge queue");
const places = role("navigation", "Places");
const main = inside(queue, role("img", "Main is green"));
const quiet = inside(queue, role("img", /^GitHub not answering since \d\d:\d\d$/));

const steps: readonly Step[] = [
  { look: main, say: "Main's chip, green" },
  { hover: main, say: "The tooltip adds when Fleet last read the forge, and the figure moves while you look" },
  { later: places, say: "GitHub stops answering Fleet" },
  { look: quiet, say: "An unplugged glyph beside main, and the dot is dimmed and hollow: this green is the last one Fleet read" },
  { hover: quiet, say: "It says since when" },
  { hover: main, say: "Main's tooltip still gives the age of the reading it holds" },
];

export const mergeLineFreshness = walk("merge-line-freshness", steps);
