// Pictures and videos a Job showed, drawn in its thread: a press opens a picture full screen, where it zooms by keys, and Close
// comes back to the thread. Over `thread-media`.

import { button, deckRow, dialog, inside, region, role, walk } from "../walk";

const WIDE = "cockpit-wide.jpg";
const viewer = dialog(WIDE);
const artifacts = region("Pane: Ledger", {
  exact: true,
});

const threadMedia = walk("thread-media", [
  {
    press: deckRow("Draw shown pictures in the thread"),
    say: "A Job that showed two pictures and two videos",
  },
  {
    look: button(`Opened ${WIDE}`, {
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
    // Its line first: a row this far up the thread has not read its video until it comes near.
    look: button("Opened cockpit-recording.mp4", {
      exact: true,
    }),
    say: "Further up, a video",
  },
  {
    look: role("group", "cockpit-recording.mp4"),
    say: "It has its first frame and its own controls",
  },
  {
    look: button("Open video"),
    say: "A video that cannot play here says so at the same height, and opens in your Mac's own player",
  },
  {
    key: "Alt+Shift+1",
    on: role("region", "Pane: Thread"),
    say: "Artifacts, here the Ledger, lists the same files under Files",
  },
  {
    press: inside(artifacts, button("cockpit-recording.mp4")),
    say: "A video's row opens in the side panel, in Bridge",
  },
  {
    look: dialog("cockpit-recording.mp4"),
    say: "It plays there with its own controls, not as a path and not on your Mac",
  },
  {
    press: button("Close"),
    say: "Close, or Esc",
  },
  {
    press: inside(artifacts, button("cockpit-wide.jpg")),
    say: "A picture's row opens the picture in the same panel",
  },
]);

export { threadMedia as "thread-media" };
