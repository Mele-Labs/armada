// The act that leaves no move to hang a door on, taken from the phone: a Session's ask answered. It
// draws the smartphone glyph, tooltip From phone; the same act in Bridge draws none. Mock data only.
//
// Session "Fix the flaky store test": open it, then the Bash calls fold. Two asks: the phone's is marked.

import { holding } from "../holding";
import { FakeSessionsFleet, asking, hosted } from "../sessions-fleet";

const SESSION = "01SESSIONPHONEDOORSAAAAAAA";

const base = holding("phone-doors", "A Session ask answered from the phone beside the same act in Bridge");

const AT = "2026-10-11T09:00:00.000Z";
const thread = {
  [SESSION]: [
    { kind: "ask" as const, id: "ask-1", at: AT, ask: asking("rm -rf build", "call-1"), state: "allowed_once" as const, via: "phone" },
    { kind: "ask" as const, id: "ask-2", at: AT, ask: asking("rm -rf dist", "call-2"), state: "allowed_once" as const },
  ],
};

export const s224PhoneDoors = new FakeSessionsFleet([hosted(SESSION, { title: "Fix the flaky store test" })], thread).scenario(base);
