// The Session thread in the new look, `?look=new`: his message, the agent's prose, its tool calls, a sketch card and the box.
// Over `thread-look`. A design mock: nothing here asserts a colour or a size.

import { button, inside, region, role, text, walk } from "../walk";

const list = region("Jobs and Sessions");
const stage = region(/^Session: Land the speed fixes/);
const thread = inside(stage, region("Thread"));

const threadLook = walk("thread-look", [
  { press: inside(list, button("Land the speed fixes")), say: "A Session opens in the Cockpit: his message is a card, and the agent's words sit on the panel" },
  { look: inside(thread, text(/The fix is to read once/)), say: "Prose at a reading measure, with room between paragraphs" },
  { press: inside(thread, text("Read, Grep")), say: "Tool calls are one mono line each, and open to the calls" },
  { look: inside(thread, button("Open sketch Store clock reads")), say: "A sketch is a card, the same as the one that needs him" },
  { type: "Use the fake clock", into: inside(stage, role("textbox", "Message")), say: "The box: the field, then the menus, then Send" },
]);

export { threadLook as "thread-look" };
