// The four things a Commands mod's row does, from the palette to what it leaves on screen: a prompt with two blanks that fills the
// dispatch sheet, an address that opens a pull request, a chain that ends on Projects, and a shell line that runs in a side panel. Mock only. Over the scenario of the same name, which reads the rows from a mod as the real window does.

import { dialog, inside, place, role, walk } from "../walk";
import type { Step } from "../walk";

const places = role("navigation", "Places");
const bar = inside(dialog("Command palette"), role("combobox"));
const request = role("textbox", "Request");

function steps(): Step[] {
  return [
    { key: "Meta+Alt+s", on: places, say: "⌥⌘S, the key a Commands mod gave one of its rows, runs it from anywhere. It has blanks, so the palette opens on the first: {pr} lists the open pull requests. The footer says Next while more is asked" },
    { press: role("option", /Cache the plan windows/), say: "Choose one" },
    { type: "security\n", into: bar, say: "{focus} is no kind Bridge lists, so it takes a typed word. This is the last, so the footer names the act: Start a Job" },
    { look: request, say: "The prompt is filled, and the dispatch sheet holds it" },
    { key: "Escape", on: request, say: "Esc puts the sheet away" },
    { key: "Meta+k", on: places, say: "⌘K" },
    { type: "on the forge\n", into: bar, say: "An address asks for its blank the same way" },
    { press: role("option", /Cache the plan windows/), say: "Choose one" },
    { look: role("region", "Pull request #1899"), say: "It opened where Bridge opens a pull request address: Reviews, on #1899" },
    { key: "Meta+k", on: places, say: "⌘K" },
    { type: "triage\n", into: bar, say: "A chain runs the rows it names, in order: Reviews, then Projects" },
    { look: place("Projects"), say: "It ended on Projects" },
    { key: "Meta+k", on: places, say: "⌘K" },
    { type: "audit dep\n", into: bar, say: "A shell line runs in a repository's checkout. None is picked, so it asks which" },
    { press: role("option", "armada", { exact: true }), say: "Run" },
    { look: role("group", "Terminal"), say: "The line runs there, and the side panel shows its output" },
  ];
}

const paletteModCommands = walk("palette-mod-commands", steps());

export { paletteModCommands as "palette-mod-commands" };
