// What a Session runs on, and a start-fresh mark once it is big or old. The model is a mono name in the head
// of every Session, the default named where none was picked. A Session over 300k of context, or past a day
// old, wears a quiet mark beside its context figure; hover or focus opens its card, with two acts: a new
// Session that opens on a short handoff, and a fork. Over the `session-fresh` scenario.

import { toSessions } from "../sessions/walk-kit";
import { button, inside, place, region, role, text, walk } from "../walk";

const sessions = region("Sessions");
const mark = button("Fresh start");
const card = role("dialog", "Fresh start");
const model = (name: string) => role("img", `Model ${name}`);

const fresh = walk("session-fresh", [
  toSessions,
  { press: inside(sessions, button(/Release notes script/)), say: "Small and new" },
  { look: model("Opus 5.5"), say: "The model its process resolved, named" },
  { hover: model("Opus 5.5"), say: "Hover for what it is" },
  { look: role("img", "Context 5%"), say: "53.8k of the window, and no mark" },
  { press: place("Sessions"), say: "Back to the list" },
  { press: inside(sessions, button(/Invoice PDF layout/)), say: "Past a day old, at 88k" },
  { look: model("Sonnet 5.5"), say: "A picked alias, resolved to its version" },
  { look: mark, say: "A quiet mark beside the context figure" },
  { hover: mark, say: "Hover for its card" },
  { press: mark, say: "A press keeps it open" },
  { look: inside(card, text("88k context")), say: "The figure" },
  { look: inside(card, text("Started 2d ago")), say: "And how long ago it started" },
  { hover: inside(card, button("Fork")), say: "Fork starts a new Session from this conversation" },
  { key: "Escape", on: mark, say: "Esc closes it" },
  { press: mark, say: "Opened again" },
  { press: inside(card, button("Fork")), say: "Fork" },
  { look: region("Session s-01FORK00"), say: "It opens at once" },
  { press: place("Sessions"), say: "Back to the list" },
  { press: inside(sessions, button(/Cart totals rounding/)), say: "612k, started hours ago" },
  { look: model("Opus 5.5"), say: "Opus again" },
  { look: role("img", "Context 61%"), say: "The context figure" },
  { press: mark, say: "Context alone sends it here" },
  { look: inside(card, text("612k context")), say: "The figure" },
  { hover: inside(card, button("New Session")), say: "A new Session that opens on a short handoff" },
  { press: inside(card, button("New Session")), say: "New Session asks the old one for its handoff" },
  { look: region("Session s-01SESSIO"), say: "The new Session opens once the handoff is written" },
  { look: inside(region("Thread"), text("Handoff from Cart totals rounding")), say: "Its first message, sent for you" },
  { look: inside(region("Thread"), text("the same rounding in the refund path")), say: "The old Session's own words" },
]);

export { fresh as "session-fresh" };
