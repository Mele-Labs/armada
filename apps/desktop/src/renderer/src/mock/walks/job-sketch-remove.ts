// Remove strikes a Drone's part out, and taking it off again puts it back; his own parts go.

import { role, walk } from "../walk";

const w = walk("job-sketch-plan", [
  { press: role("group", "Writer"), say: "Pick a part of the Drone's" },
  { press: role("button", "Remove", { exact: true }), say: "Remove" },
  { look: role("group", "Struck out, Writer"), say: "It is struck out and stays, so the Drone sees what he did not want" },
  { press: role("button", "Put back", { exact: true }), say: "Put it back by taking it off again" },
]);

export { w as "job-sketch-remove" };
