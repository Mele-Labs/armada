// A pull request's description plays its attachments where it is read: a video is a poster until pressed, then plays with controls;
// a picture is a thumbnail that opens larger. Over `pr-attachments`.

import { button, inside, region, role, tab, walk } from "../walk";

const queue = region("Merge queue");
const pull = role("dialog", "Pull request #1895");

const prAttachments = walk("pr-attachments", [
  { press: inside(queue, button(/^#1895/)), say: "A pull request with a Demo section" },
  { look: inside(pull, tab("Conversation")), say: "The conversation, description first" },
  { look: inside(pull, button("Play video")), say: "The bare attachment address is a poster. Nothing has been fetched" },
  { press: inside(pull, button("Play video")), say: "Pressed: the video is fetched and plays with its own controls" },
  { look: inside(pull, button("The row")), say: "The picture is a thumbnail" },
  { press: inside(pull, button("The row")), say: "It opens larger" },
  { key: "Escape", on: role("dialog", "The row"), say: "Esc closes it" },
]);

export { prAttachments as "pr-attachments" };
