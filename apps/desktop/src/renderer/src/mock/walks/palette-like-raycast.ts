// The palette like Raycast's, mock only. ⌘K opens on Pinned and Recent; Jobs from two repositories with their state; a
// pull request found by its number and opened in Reviews; a task filed from the bar; ⌘K on a row opens its acts (Pause on
// a Job, Pin on a task); Projects' own acts head the results; a command with a blank asks for it in a second step. Over the
// scenario of the same name.

import { dialog, inside, place, role, walk } from "../walk";
import type { Step } from "../walk";

const places = role("navigation", "Places");
const bar = inside(dialog("Command palette"), role("combobox"));

function steps(): Step[] {
  return [
    { key: "Meta+k", on: places, say: "⌘K, with nothing typed: Pinned, then Recent. Each Job row names its repository, its state as a glyph and the pull request it holds" },
    { look: role("option", /^Cache key/), say: "Cache key is pinned, and the glyph says it waits on him" },
    { type: "migration", into: bar, say: "Jobs from both repositories, one at work and one paused. A number or a repository name finds a row as a title does" },
    { key: "Meta+k", on: bar, say: "⌘K on a row opens its acts inside the palette, and the palette stays up. Esc goes back to the results with the words kept" },
    { press: role("option", "Pause", { exact: true }), say: "Pause. Stop asks first" },
    { key: "Meta+k", on: places, say: "⌘K again. The state glyph on that Job is paused now" },
    { type: "#1899\n", into: bar, say: "# and a number finds the pull request, and Enter opens it in Reviews" },
    { look: role("region", "Pull request #1899"), say: "Reviews, on #1899" },
    { key: "Meta+k", on: places, say: "⌘K" },
    { type: "Check the pin order", into: bar, say: "Words" },
    { key: "Tab", on: bar, say: "Tab steps Job, Autopilot, Task, Project" },
    { key: "Tab", on: bar, say: "Task" },
    { key: "Enter", on: bar, say: "It asks which project" },
    { press: role("option", "Palette"), say: "Filed in Palette" },
    { press: place("Projects"), say: "Projects" },
    { key: "Meta+k", on: places, say: "⌘K on Projects" },
    { type: "task", into: bar, say: "Projects' own acts head the results, in a block titled with the page. Tasks come after" },
    { type: "tick a task", into: bar, say: "A task row" },
    { key: "Meta+k", on: bar, say: "Its acts: Open, Dispatch, Tick and Pin" },
    { press: role("option", "Pin", { exact: true }), say: "Pin keeps the palette up" },
    { key: "Escape", on: bar, say: "Esc closes it" },
    { key: "Meta+k", on: places, say: "Empty again: the task is under Pinned" },
    { look: role("option", /^Tick a task from the palette/), say: "Pinned" },
    { type: "security", into: bar, say: "A row from a Commands mod, with its alias and its key" },
    { key: "Enter", on: bar, say: "Its prompt has a blank, {pr}. The second step asks for the pull request" },
    { press: role("option", /Bump the manifest icons/), say: "Choose one" },
    { look: role("textbox", "Request"), say: "The Job's prompt is filled, and the dispatch sheet holds it" },
  ];
}

const paletteLikeRaycast = walk("palette-like-raycast", steps());

export { paletteLikeRaycast as "palette-like-raycast" };
