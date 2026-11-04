// Pictures and videos a Job showed, drawn in its thread: a press opens a picture full screen, where it zooms by keys, and Close
// comes back to the thread. Over `thread-media`.

import { button, deckRow, dialog, role, walk } from "../walk";

const WIDE = "cockpit-wide.jpg";
const viewer = dialog(WIDE);

const threadMedia = walk("thread-media", [
  {
    press: deckRow("Draw shown pictures in the thread"),
    say: "A Job that showed two pictures and two videos",
  },
  {
    look: button(WIDE, {
      exact: true,
    }),
    say: "Each draws under its line at one height, so the thread does not move as they arrive",
  },
  {
    press: button(WIDE, {
      exact: true,
    }),
    say: "A press opens the picture full screen",
  },
  {
    key: "+",
    on: viewer,
    say: "+ zooms in. So do a pinch, a wheel and a double click",
  },
  {
    key: "ArrowRight",
    on: viewer,
    say: "The arrows pan while it is zoomed",
  },
  {
    key: "0",
    on: viewer,
    say: "0 fits it again",
  },
  {
    press: button("Close", {
      exact: true,
    }),
    say: "Close, or Esc. Back in the thread",
  },
  {
    look: role("group", "cockpit-recording.mp4"),
    say: "A video has its first frame and its own controls",
  },
  {
    look: button("Open video"),
    say: "A video that cannot play here says so at the same height, and opens in your Mac's own player",
  },
]);

export { threadMedia as "thread-media" };
