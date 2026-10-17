// Pause on Flight control for a Session that is working: it is marked paused in the list, the tile reads Resume, Resume puts it back,
// and the key P does the same from the keyboard. Over `pausing-a-session`.

import { button, inside, region, role, walk } from "../walk";

const list = region("Work", { exact: true });
const controls = region("Flight controls");
const paused = inside(list, role("img", "Paused"));

const pausingASession = walk("pausing-a-session", [
  { press: inside(list, button("Pin the store clock")), say: "A Session working, open on the stage" },
  { press: inside(controls, button("Pause")), say: "Pause, beside the other flight controls" },
  { look: paused, say: "Its row stays in the list with the paused mark" },
  { look: inside(controls, button("Resume")), say: "The tile reads Resume now" },
  { press: inside(controls, button("Resume")), say: "Resume takes a slot again" },
  { key: "p", on: controls, say: "P pauses it from the keyboard" },
  { look: paused, say: "Paused again" },
  { key: "p", on: controls, say: "P resumes it" },
]);

export { pausingASession as "pausing-a-session" };
