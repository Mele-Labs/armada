// A Session's agent wireframes screens with `show_sketch`: a desktop window whose steps ring a button and bring up a modal, a web
// app in a browser frame, and a flow of two phone screens joined by an arrow. Over the `screen-wireframes` scenario.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { FLOW_TITLE, MEMBERS_TITLE, OPEN_TITLE } from "../sessions/screens";
import { kit, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const { message, thread, close } = kit(false);
  const panel = (title: string) => dialog(`Sketch ${title}`);
  const drawn = (title: string) => inside(thread, button(`Open sketch ${title}`));
  const play = (title: string) => inside(panel(title), button(/Play the steps|Stop the steps|Next step/));
  return [
    { press: inside(region("Jobs"), button("Screen wireframes")), say: "A Job whose agent can draw screens" },
    { type: "Wireframe the session screen, team settings and the sign in\n", into: message, say: "Asked for the screens" },
    { look: drawn(OPEN_TITLE), say: "The desktop window as a card in the thread: a sheet of its own, drawn from a layout tree" },
    { press: drawn(OPEN_TITLE), say: "Opened beside the thread" },
    { look: inside(panel(OPEN_TITLE), text("Inject the clock")), say: "Side nav, a rail of icon buttons, the thread over its composer, and two panels. The agent never placed any of it" },
    { look: inside(panel(OPEN_TITLE), role("group", "Artifacts", { exact: true })), say: "Step one rings the Artifacts button and the rest of the screen stands back" },
    { look: inside(panel(OPEN_TITLE), role("group", "Artifact finder", { exact: true })), say: "Step two brings up the modal over the window, with a scrim" },
    { look: play(OPEN_TITLE), say: "Both steps loop; Restart takes it back to the first" },
    { press: close(panel(OPEN_TITLE)), say: "Close" },
    { press: drawn(MEMBERS_TITLE), say: "A web app in a browser frame, for any product" },
    { look: inside(panel(MEMBERS_TITLE), text("app.tallyboard.example/settings/team")), say: "Tab and address bar, then the top nav, side nav, tabs, a table and a form from the same kit" },
    { look: inside(panel(MEMBERS_TITLE), text("Save changes")), say: "Buttons, selects, a toggle and a checkbox are controls drawn as wireframes, never live ones" },
    { press: close(panel(MEMBERS_TITLE)), say: "Close" },
    { press: drawn(FLOW_TITLE), say: "A flow: two phone screens and an arrow between them" },
    { look: inside(panel(FLOW_TITLE), role("img", "Sign in", { exact: true })), say: "The arrow joins whole screens and carries its words" },
    { look: play(FLOW_TITLE), say: "Step one rings the Sign in button, step two sends the dot across and lights Home" },
    { press: close(panel(FLOW_TITLE)), say: "Close" },
  ];
}

const wireframes = walk("screen-wireframes", [toSessions, ...steps()]);

export { wireframes as "screen-wireframes" };
