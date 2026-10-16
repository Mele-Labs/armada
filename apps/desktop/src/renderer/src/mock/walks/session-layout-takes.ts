// Where an open Session keeps Flight control and the Field, so the thread has height on a 14" screen. The takes are the page's `?take=`
// (`cockpit/deck/session-take.ts`): right, fold and keys, and none for today's. Only the steps that reach the layout, over the
// `one-session-view` scenario; `oneSessionView` walks the whole Session.

import type { Step } from "../walk";
import { deckRow, region, walk } from "../walk";

const take = typeof window === "undefined" ? null : new URLSearchParams(window.location.search).get("take");
const unfold: Step[] =
  take === "fold"
    ? [{ key: "f", on: region(/^Session: /), say: "F opens Flight control and the Field together. A press on either band opens that one" }]
    : [];

export const sessionLayoutTakes = walk("one-session-view", [
  { press: deckRow("Retry the slot lease"), say: "A Session open. The thread is the height in question" },
  { look: region("Flight controls"), say: "Flight control" },
  { look: region("Field"), say: "The Field" },
  ...unfold,
]);
