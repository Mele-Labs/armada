// An item that waits on the person for a procedure: its row wears the Steps glyph, and a press opens the
// steps beside the thread in place of the link. The link is the key at the top, each step has a tick that
// is kept, a step the agent got wrong is edited, dropped or added, and Send takes the corrections with
// the reply, and with the box empty the key reads Done, whatever was ticked. Over the `needs-you-steps` scenario.

import { button, dialog, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const TITLE = "Register the OAuth app";

function steps(): Step[] {
  const sessions = region("Jobs", { exact: true });
  const needs = region("Needs you");
  const panel = dialog(TITLE);
  return [
    toSessions,
    { press: inside(sessions, button(/Add single sign-on/)), say: "Opened" },
    { press: inside(needs, button(/^Needs you/)), say: "Needs you, raised from the composer" },
    { hover: inside(needs, role("img", "Steps")), say: "A procedure wears the Steps glyph" },
    { hover: inside(needs, role("img", "Reply")), say: "An ordinary item keeps its own" },
    { press: inside(needs, button(`Open ${TITLE}`)), say: "A press on the row opens its steps, not the link" },
    { look: inside(panel, button(/auth\.example\.com\/apps\/new/)), say: "The link is one key at the top" },
    { hover: inside(panel, button("Open in browser")), say: "A second key opens it in the browser" },
    { look: inside(panel, role("checkbox", "Name it Armada")), say: "Each step has a tick" },
    { press: inside(panel, role("checkbox", "Name it Armada")), say: "The first step done" },
    { press: inside(panel, role("checkbox", "Tick Enable Device Flow")), say: "The third done, the second left" },
    { press: inside(panel, role("button", /^Close( panel)?( Esc)?$/)), say: "Closed" },
    { press: inside(needs, button(`Open ${TITLE}`)), say: "Opened again" },
    { look: inside(panel, role("checkbox", "Tick Enable Device Flow")), say: "The ticks are kept" },
    { press: inside(panel, role("button", "Edit step 2")), say: "A step the agent got wrong is edited in place" },
    { type: "Homepage and callback both http://127.0.0.1:8470\n", into: inside(panel, role("textbox", "Step 2")), say: "Enter saves it" },
    { hover: inside(panel, role("img", /^Step 2 was/)), say: "A small mark, and on hover the agent's own words" },
    { press: inside(panel, role("button", "Drop step 4")), say: "A step that does not belong is dropped" },
    { look: inside(panel, role("button", "Bring back step 4")), say: "It stays in the list, struck through, and can be brought back" },
    { press: inside(panel, role("button", "Add step")), say: "A step the agent missed is added at the end" },
    { type: "Leave the Client secret alone\n", into: inside(panel, role("textbox", "Step 7")), say: "Worded and saved" },
    { look: inside(panel, button("Done")), say: "Done is ready with nothing more ticked and the reply box empty: the corrections go with it" },
  ];
}

const played = walk("needs-you-steps", steps());

export { played as "needs-you-steps" };
