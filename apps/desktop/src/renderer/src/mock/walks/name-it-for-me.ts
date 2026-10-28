// A Job named by Fleet on request: the wand after the title, busy for the second the call takes, then the short name
// where the long one was. The pencil still edits it. Over the `name-it-for-me` scenario.

import { button, inside, region, role, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const frame = region("Job s-7c41d9a2");
const long = "I think the Checks tab should sit beside Bays and open in the spine instead of";
const short = "Checks tab beside Bays in the spine";
const wand = inside(frame, button("Name it for me"));

const nameItForMe = walk("name-it-for-me", [
  toSessions,
  { press: inside(region("Jobs", { exact: true }), button(long)), say: "The Job's title is the first line of the note it started from" },
  { look: inside(frame, button(`${long}, rename`)), say: "Too long to be a name" },
  { hover: wand, say: "The wand after the title has Fleet name it" },
  { press: wand, say: "Pressed" },
  { look: inside(frame, button("Name it for me", { exact: true })), say: "The wand is busy while Fleet answers" },
  { look: inside(frame, button(`${short}, rename`)), say: "The short name is the title" },
  { press: inside(frame, button(`${short}, rename`)), say: "A press on the title edits it where it stands, as before" },
  { look: inside(frame, role("textbox", "Job name")), say: "The title is a field" },
]);

export { nameItForMe as "name-it-for-me" };
