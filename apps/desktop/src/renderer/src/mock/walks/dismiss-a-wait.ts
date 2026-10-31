// Each row of the Waiting band has a cross: on a pull request it stops counting that pull request, which merges as before; on a watch
// it stops the watch. With nothing left, the band goes and the row reads idle. Over `row-states`.

import { button, deckRow, inside, region, role, walk } from "../walk";

const waiting = region("Waiting", { exact: true });

const dismissAWait = walk("row-states", [
  { press: deckRow("Raise the upload limit"), say: "A Job waiting on a deploy watch and a pull request" },
  { press: inside(waiting, button("Waiting")), say: "Open its band" },
  { hover: inside(waiting, button("Stop waiting on it: #409 to auto-merge once Checks pass")), say: "Each row has a cross" },
  { press: inside(waiting, button("Stop waiting on it: #409 to auto-merge once Checks pass")), say: "On a pull request it stops counting it. #409 still merges" },
  { look: inside(waiting, role("list", "Waiting on")), say: "Only the watch is left" },
  { press: inside(waiting, button("Stop the watch: Held open by Staging deploy")), say: "On a watch it stops the watch" },
  { hover: inside(deckRow("Raise the upload limit"), role("img", "Idle", { exact: true })), say: "Nothing left to wait on: the row reads idle" },
]);

export { dismissAWait };
