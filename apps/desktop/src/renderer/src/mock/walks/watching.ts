// Delivery draws a Job's watches in a Watching block below its pull requests, the agent's verdict under the reading it weighed. A press opens a
// watch in the side panel: its readings, its parts with the one set aside struck through, its command. A Job a live watch alone holds open wears
// the watch's glyph on its Cockpit row and in its header, named Held open by on hover, and Stop on that watch lets it go. Over `watching`.

import { button, deckRow, dialog, inside, region, role, tab, walk } from "../walk";

const landing = region("Delivery", { exact: true });
const watchingBlock = inside(landing, region("Watching"));
const jobs = region("Jobs", { exact: true });
const panel = dialog("Watch QA deploy");
const staging = dialog("Watch Staging deploy");

const watching = walk("watching", [
  { press: deckRow("Ship the rate limit to QA"), say: "A Job in the middle of a QA deploy" },
  { look: watchingBlock, say: "Watching, below its pull request: each watch's state, name, last sentence and age, and under the failed one what the agent made of it" },
  { hover: inside(watchingBlock, role("img", "Failed")), say: "A state is its glyph, named on hover" },
  { press: inside(landing, button("Watch QA deploy")), say: "The failed watch opens in the side panel" },
  { look: panel, say: "Its readings over time, newest first, each verdict under the reading it weighed, and the link out on the heading's line" },
  { press: inside(panel, tab("Parts")), say: "Its parts: the one set aside is struck through, with the sentence that set it aside" },
  { press: inside(panel, tab("Command")), say: "The whole command Fleet runs for the Job, and how often" },
  { key: "Escape", on: panel, say: "Esc closes it" },
  { hover: inside(jobs, role("img", "Held open by Staging deploy")), say: "A Job whose turn ended with a deploy still running: Held open by Staging deploy" },
  { press: deckRow("Raise the upload limit"), say: "Its header wears the same glyph" },
  { hover: inside(region(/^Job /), role("img", /^Held open by Staging deploy/)), say: "Held open by Staging deploy" },
  { press: inside(landing, button("Watch Staging deploy")), say: "The watch that holds it" },
  { press: inside(staging, button("Stop")), say: "Stop ends the watch, and nothing holds the Job open" },
  { key: "Escape", on: staging, say: "Esc" },
  { look: watchingBlock, say: "Staging deploy reads Stopped" },
]);

export { watching };
