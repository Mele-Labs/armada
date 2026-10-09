// What a Session's Artifacts rows open: each in the side panel, a picture fit to it, a file's text, and a
// page or a Doc in a frame. Over the `session-ledger` scenario, whose "Write up the store clock" Session
// holds one of each.

import { button, dialog, inside, region, role, text, walk } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const { ledger, opened, row, close } = kit(false);
  const sessions = region("Sessions");
  const panel = (title: string) => dialog(title);
  return [
    { press: inside(sessions, button("Write up the store clock")), say: "A Session that wrote a file, looked at a screenshot and published a page" },
    ...opened([
      { press: inside(ledger, button("Pictures")), say: "Pictures start out of the list; press for them" },
      { look: inside(ledger, role("img", "Looked at")), say: "The screenshot it read" },
    ]),
    ...row("Looked at ledger-screenshot.png", "A picture opens in the panel"),
    { look: inside(panel("ledger-screenshot.png"), role("img", "ledger-screenshot.png")), say: "The picture, fit to the panel" },
    { press: close(panel("ledger-screenshot.png")), say: "Close" },
    ...row("File written store-clock.md", "A file it wrote opens in the panel"),
    { look: inside(panel("store-clock.md"), text("The store reads the clock once per write.")), say: "Its markdown, rendered" },
    { press: close(panel("store-clock.md")), say: "Close" },
    ...row("Published page Store clock findings", "A page it published opens in the panel"),
    { look: inside(panel("Store clock findings"), region("Page")), say: "The page in a frame, where Bridge's window lays a web view" },
    { look: inside(panel("Store clock findings"), button("Open in browser")), say: "A press for the browser, for a page that refuses to embed" },
    { press: close(panel("Store clock findings")), say: "Close" },
    ...row("Doc Store clock write-up", "A Doc opens the same way"),
    { look: inside(panel("Store clock write-up"), region("Page")), say: "The Doc in a frame" },
  ];
}

const panel = walk("session-ledger", [toSessions, ...steps()]);

export { panel as "session-ledger-panel" };
