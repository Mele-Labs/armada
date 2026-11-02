// A Session's agent draws a screen as hi-fi parts: HTML in the app's own classes over the app's own stylesheets, each part in
// a sealed frame on the sketch's canvas. The card in the thread, the sketch open, a zoom, a note on one element, and what a
// part was drawn with. Over the `hi-fi-sketch` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";
import { ONE_CARD_TITLE } from "../sessions/hi-fi";

function steps(): Step[] {
  const { message, thread, close } = kit(false);
  const panel = dialog(`Sketch ${ONE_CARD_TITLE}`);
  const frame = (title: string) => inside(panel, role("group", `Frame ${title}`, { exact: true }));
  return [
    { press: inside(region("Jobs", { exact: true }), button("One card sketches")), say: "A Job whose agent draws screens" },
    { type: "Sketch the one card in every list\n", into: message, say: "Asked for the card" },
    { look: inside(thread, button(`Open sketch ${ONE_CARD_TITLE}`)), say: "The sketch is a card in the thread" },
    { press: inside(thread, button(`Open sketch ${ONE_CARD_TITLE}`)), say: "Opened beside the thread" },
    { look: frame("Issues"), say: "Each part is the app's own markup in a frame, drawn with its stylesheets" },
    { key: "+", on: inside(panel, role("group", "Sketch", { exact: true })), say: "Zoom in over a frame: the text stays sharp and the canvas still pans" },
    { context: frame("Pull requests"), say: "Right press on an element inside a frame" },
    { press: role("menuitem", "Add note"), say: "Add note" },
    { type: "Keep the failed tone this red\n", into: role("textbox", "Note"), say: "The note is about that element, and its number stands on it" },
    { look: inside(panel, text(/tokens\.css, base\.css, JobCard\.css/)), say: "Under each part, the stylesheets it was drawn with" },
    { look: inside(panel, text("No stylesheet of the app")), say: "A part drawn with none says so" },
    { press: close(panel), say: "Close" },
  ];
}

const hifi = walk("hi-fi-sketch", [toSessions, ...steps()]);

export { hifi as "hi-fi-sketch" };
