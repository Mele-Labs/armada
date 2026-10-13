// A failing pull request a Session holds. While the Session works on it there is no call. When it goes
// idle with nothing sent since the failure, the call comes up with the Session's thread under it, and
// Poke sends it a line and takes the call away. Over `pr-call-owner-working`.

import { button, inside, region, role, tab, text, walk } from "../walk";
import { toSessions } from "../sessions/walk-kit";

const cockpit = region("Dashboard");
const call = region(/^Pull request: #1904/);

const prCallOwnerWorking = walk("pr-call-owner-working", [
  { look: tab("Your move"), say: "The checks on #1904 failed six minutes ago. The Session that holds it is at work, so Your move carries no dot for it" },
  { later: cockpit, say: "The Session ends its turn. Nothing has been sent to it since the failure" },
  { look: call, say: "Now the failure is a call" },
  { look: inside(call, role("group", /^Session Pin the Bridge clock/)), say: "The Session's own thread is under the facts: what it did, and where it stopped" },
  { look: inside(call, text("desktop_test is failing on a clock the Bridge tests also read.")), say: "Its last line is the failure it is looking at" },
  { look: button(/Poke/), say: "Poke sends it the failure" },
  { key: "2", on: cockpit, say: "2 pokes it" },
  { look: tab("Your move"), say: "The call is gone: the Session has been told, and is working" },
  toSessions,
  { press: inside(region("Sessions"), button(/Pin the Bridge clock/)), say: "Its own page" },
  { look: text("Pull request #1904's checks failed. Address them and push to the pull request."), say: "The Poke is the last line of its thread" },
]);

export { prCallOwnerWorking as "pr-call-owner-working" };
