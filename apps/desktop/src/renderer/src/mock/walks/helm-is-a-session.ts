// Helm is a hosted Session pinned to the dock, one per repository. Its thread is drawn as a Session's
// is: tool calls fold into groups, the working mark closes a turn that is running, and Stop sits beside
// it. Once Helm writes it holds a slot and a branch, and a pull request it opens, as chips under the
// dock's head; a subagent it runs opens in a sheet, and a question it asks is answered in the thread.
// A thought for other work keeps its card and its "Sent to" row. Over `helm-is-a-session`; each `later` is the
// turn's next step.

import { button, inside, region, role, text, walk } from "../walk";

const helm = role("complementary", "Helm");
const box = inside(helm, role("textbox", "Ask Helm"));
const held = inside(helm, role("group", "Held by Helm"));
const thread = inside(helm, region("Thread"));
const ask = inside(helm, role("article", "Waiting on you"));
const subagent = role("dialog", "Subagent Find the other clock callers");

const helmIsASession = walk("helm-is-a-session", [
  { press: button("Helm"), say: "Helm's dock on armada" },
  { look: inside(thread, text("Conversation compacted")), say: "An earlier conversation, compacted, as a Job's is" },
  { look: inside(thread, text("Read ci.md")), say: "Its tool calls are one folded row" },

  { type: "can you pin the clock in the ledger test?", into: box, say: "A request" },
  { key: "Enter", on: box, say: "↵ sends it" },
  { look: inside(thread, text("Reading multiple files")), say: "Helm reads: the calls fold into one row" },
  { look: inside(thread, role("img", "Working")), say: "The working mark sits on the last row" },
  { hover: inside(thread, button("Stop")), say: "Stop, beside the working mark, as a Job's is" },

  { later: inside(thread, button("Stop")), say: "Helm writes" },
  { look: inside(thread, region("Slot leased")), say: "The first write leases a slot and cuts a branch, in the thread where it happened" },
  { look: inside(held, role("img", "Worktree slot 8")), say: "The slot is the chip on the branch's chip under the dock's head" },
  { hover: inside(held, button("Open Branch helm/pin-ledger-clock")), say: "Named on hover" },

  { later: inside(thread, button("Stop")), say: "Helm runs a subagent" },
  { press: inside(held, button(/^Open Subagent Find the other clock callers/)), say: "Its chip opens its thread" },
  { look: inside(subagent, region("Thread")), say: "The subagent's own thread, drawn as a Job's is" },
  { look: inside(subagent, text("Grep Clock::now crates")), say: "What it has called so far" },
  { press: inside(subagent, button(/^Close/)), say: "Back to Helm" },

  { later: inside(thread, button("Stop")), say: "The subagent reports, and Helm asks" },
  { look: inside(ask, text("Which clock should the ledger test read?")), say: "The question is in the thread, with its options" },
  { press: inside(ask, role("radio", "Pinned instant")), say: "An option" },
  { press: inside(ask, button("Answer")), say: "Answered" },
  { look: inside(thread, text("Going with Pinned instant.")), say: "Helm carries on from the answer" },
  { look: inside(held, button(/^Open Pull request #2231/)), say: "The pull request it opened joins the chips, with its Checks" },
  { hover: inside(thread, role("link", "#2231")), say: "And is linked in the reply" },

  { type: "I wish that the sessions page would move sessions into a new status that says they are just waiting for work to land", into: box, say: "A thought for other work" },
  { key: "Enter", on: box, say: "Sent" },
  { look: inside(helm, role("group", "Send to")), say: "The card is Helm's own, as before" },
  { key: "Enter", on: box, say: "Press ↵" },
  { look: inside(thread, button("Sent to Job Sessions list views")), say: "The words stay in the thread, with where they went" },

]);

export { helmIsASession as "helm-is-a-session" };
