// What a Job's Artifacts rows open, from its Pages, Files and Docs groups: each in the side panel, a picture fit to it,
// a file's text, and a page or a Doc in a frame, with a note sent to it beside them. Over the `session-ledger` scenario, whose "Write up the store clock" Job
// holds one of each.

import { button, dialog, inside, region, role, walk } from "../walk";
import type { Step } from "../walk";
import { kit, toSessions } from "../sessions/walk-kit";

function steps(): Step[] {
  const { artifacts, opened, row, close } = kit(false);
  const sessions = region("Jobs", { exact: true });
  const panel = (title: string) => dialog(title);
  return [
    { press: inside(sessions, button("Write up the store clock")), say: "A Job that wrote a file, looked at a screenshot, published a page and made a Doc" },
    ...opened([
      { look: inside(artifacts, region("Pages", { exact: true })), say: "The page it published" },
      { look: inside(artifacts, region("Files", { exact: true })), say: "The file it wrote and the screenshot, in Files" },
      { look: inside(artifacts, region("Docs", { exact: true })), say: "The Doc it made" },
      { look: inside(artifacts, region("Notes", { exact: true })), say: "A note you sent it: its words, the component and the screen, and its state" },
      { look: inside(artifacts, button("Picture of The clock legend reads too small beside the chart")), say: "The note's picture, small, which opens large" },
    ]),
    ...row("ledger-screenshot.png", "A picture opens in the panel"),
    { look: inside(panel("ledger-screenshot.png"), role("img", "ledger-screenshot.png")), say: "The picture, fit to the panel" },
    { press: close(panel("ledger-screenshot.png")), say: "Close" },
    ...row("store-clock.md", "A file it wrote opens in the panel"),
    { look: inside(panel("store-clock.md"), role("group", "Editor store-clock.md", { exact: true })), say: "Its text, in the editor" },
    { press: close(panel("store-clock.md")), say: "Close" },
    ...row("Store clock findings", "A page it published opens in the panel"),
    { look: inside(panel("Store clock findings"), region("Page")), say: "The page in a frame, where Bridge's window lays a web view" },
    { look: inside(panel("Store clock findings"), button("Open in browser")), say: "A press for the browser, for a page that refuses to embed" },
    { press: close(panel("Store clock findings")), say: "Close" },
    ...row("Store clock write-up", "A Doc opens the same way"),
    { look: inside(panel("Store clock write-up"), region("Page")), say: "The Doc in a frame" },
  ];
}

const panel = walk("session-ledger", [toSessions, ...steps()]);

export { panel as "session-artifacts-panel" };
