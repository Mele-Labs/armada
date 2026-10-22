// Any file opens in the side panel's code editor, where it is edited and saved: settings.json from Settings, a file a
// Session's agent showed, and whatever ⌘O picks. A save that finds the file changed on disk asks which stands. A markdown file
// offers Preview, a rendered view beside it that follows the edit.
// Over the `file-panel` scenario.

import { button, dialog, inside, place, region, role, text, toSystemPage, walk } from "../walk";
import type { Step } from "../walk";

const editor = (name: string) => inside(dialog(name), role("group", `Editor ${name}`, { exact: true }));
const close = (name: string) => inside(dialog(name), role("button", /^Close/));

function steps(): Step[] {
  const settings = editor("settings.json");
  return [
    ...toSystemPage("Settings", "Settings"),
    { press: button("Open settings.json"), say: "Open settings.json" },
    { look: settings, say: "It opens in the panel, in the editor" },
    { type: '@9 ,\n  "sounds": false', into: settings, say: "A line added inside the braces" },
    { look: inside(dialog("settings.json"), button("Save")), say: "Save appears while the file has an edit" },
    { key: "Meta+s", on: settings, say: "⌘S saves" },
    { look: settings, say: "Save is gone" },
    { type: '@10 ,\n  "autosave": true', into: settings, say: "Another line" },
    { later: settings, say: "Another program changes the file" },
    { key: "Meta+s", on: settings, say: "⌘S" },
    { look: inside(dialog("settings.json"), text("Changed on disk")), say: "The file moved, so nothing was written. Both are drawn" },
    { look: inside(dialog("settings.json"), role("group", "Editor settings.json on disk")), say: "What is on disk now" },
    { press: inside(dialog("settings.json"), button("Keep mine")), say: "Keep mine writes the edit over it" },
    { look: settings, say: "Saved" },
    { key: "Meta+o", on: role("navigation", "Places"), say: "⌘O asks for a file" },
    { look: editor("clock.rs"), say: "It opens with Rust colours" },
    { key: "Meta+o", on: role("navigation", "Places"), say: "⌘O again" },
    { look: inside(dialog("clock-blocks.png"), role("img", "clock-blocks.png")), say: "A picture is shown, not edited" },
    { press: close("clock-blocks.png"), say: "Close" },
    { press: place("Cockpit"), say: "Cockpit" },
    { press: inside(region("Jobs", { exact: true }), button("Store clock notes")), say: "A Session whose agent showed a file" },
    { press: inside(region("Thread"), button("Opened store-clock.md")), say: "Its row in the thread" },
    { look: editor("store-clock.md"), say: "The panel opens on it, in the editor" },
    { press: inside(dialog("store-clock.md"), button("Preview")), say: "A markdown file offers Preview" },
    { look: inside(dialog("Preview"), role("region", "Rendered markdown")), say: "It opens beside the file, drawn as markdown" },
    { type: "@3  Typed here", into: editor("store-clock.md"), say: "A few words typed in the file" },
    { look: inside(dialog("Preview"), text("Typed here")), say: "They are in the preview as they are typed" },
    { press: inside(dialog("store-clock.md"), button("Preview")), say: "Preview again" },
    { press: close("store-clock.md"), say: "Close" },
    { look: inside(region("Thread"), role("link", "docs/notes/store-clock.md")), say: "The agent's message names the file in backticks, and that is a link" },
    { look: inside(region("Thread"), text("docs/notes/gone.md")), say: "A path that is not a file in the checkout stays plain" },
    { press: inside(region("Thread"), role("link", "docs/notes/store-clock.md")), say: "Press the path" },
    { look: editor("store-clock.md"), say: "The panel opens on it" },
  ];
}

const filePanel = walk("file-panel", steps());

export { filePanel as "file-panel" };
