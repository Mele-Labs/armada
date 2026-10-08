// A picture a Session looked at, on its ledger under Artifacts beside what it wrote. Over the
// `session-ledger` scenario, whose "Write up the store clock" Session holds a screenshot it read.

import { button, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { kit } from "../sessions/walk-kit";

function steps(): Step[] {
  const { ledger, opened } = kit(false);
  const sessions = region("Sessions");
  return [
    { press: inside(sessions, button("Write up the store clock")), say: "A Session that wrote a file and looked at a screenshot" },
    ...opened([
      { look: inside(ledger, role("img", "File written")), say: "The file it wrote" },
      { look: inside(ledger, role("img", "Looked at")), say: "The screenshot it read, named by its file, which opens on this machine" },
    ]),
  ];
}

const images = walk("session-ledger", steps());

export { images as "session-ledger-images" };
