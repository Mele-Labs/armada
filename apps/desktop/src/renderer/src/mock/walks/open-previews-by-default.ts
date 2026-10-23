// A markdown file opens with its preview showing once the person chose that, and Esc closes the preview and the file panel together.
// Over the `file-panel` scenario.

import { button, dialog, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const editor = inside(dialog("store-clock.md"), role("group", "Editor store-clock.md", { exact: true }));
const preview = dialog("Preview");

function opened(): Step[] {
  return [
    { press: inside(region("Thread"), button("Opened store-clock.md")), say: "Its row in the thread" },
    { look: editor, say: "The panel opens on the file" },
  ];
}

const openPreviewsByDefault = walk("file-panel", [
  { press: place("Cockpit"), say: "Cockpit" },
  { press: inside(region("Jobs", { exact: true }), button("Store clock notes")), say: "A Session whose agent showed a file" },
  ...opened(),
  { press: inside(dialog("store-clock.md"), button("Preview")), say: "Preview" },
  { look: inside(preview, role("region", "Rendered markdown")), say: "The preview opens beside the file" },
  { press: inside(preview, role("switch", "Open previews by default")), say: "Open previews by default, in the preview's head" },
  { key: "Escape", on: editor, say: "Esc" },
  { look: text("Store clock notes"), say: "The preview and the file panel are both closed" },
  ...opened(),
  { look: inside(preview, role("region", "Rendered markdown")), say: "The file opens with its preview already showing" },
]);

export { openPreviewsByDefault as "open-previews-by-default" };
