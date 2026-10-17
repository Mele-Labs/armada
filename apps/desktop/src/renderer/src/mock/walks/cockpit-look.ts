// The Cockpit's panels and rows in the new look, `?look=new`: one frame and header, calls, rows, the repository menu.
// Over `cockpit-list-by-repository`. A design mock: nothing here asserts a colour or a size.

import { button, inside, region, text, walk } from "../walk";

const list = region("Jobs and Sessions");
const calls = region("Incoming calls");

const cockpitLook = walk("cockpit-list-by-repository", [
  { look: list, say: "Every panel has one frame and one header" },
  { hover: inside(list, text("The retry loop")), say: "A row under the pointer rises" },
  { hover: inside(calls, text("Prune the stale branches")), say: "A call reads kind, subject, age" },
  { press: inside(calls, text("Prune the stale branches")), say: "The opened Session is lit in the list" },
  { press: button("Repository"), say: "The repository menu" },
]);

export { cockpitLook as "cockpit-look" };
