// The four things a Commands mod's row does, from the palette to what it leaves on screen: a prompt with two blanks that fills the
// dispatch sheet, an address that opens a pull request, a chain that ends on Projects, and a shell line whose row says it cannot
// run. Mock only. Over the scenario of the same name, which reads the rows from a mod as the real window does.

import { dialog, inside, place, role, walk } from "../walk";
import type { Step } from "../walk";

const places = role("navigation", "Places");
const bar = inside(dialog("Command palette"), role("combobox"));
const request = role("textbox", "Request");

function steps(): Step[] {
  return [
    { key: "Meta+k", on: places, say: "⌘K. The rows of a Commands mod sit with the rest, each with the glyph of what it does" },
    { type: "security pass\n", into: bar, say: "A prompt with blanks asks for each in the palette. {pr} lists the open pull requests" },
    { press: role("option", /Cache the plan windows/), say: "Choose one" },
    { type: "security\n", into: bar, say: "{focus} is no kind Bridge lists, so it takes a typed word" },
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
    { type: "audit dep", into: bar, say: "A shell line has no terminal to run in yet. Its row says so, and Enter does nothing" },
    { look: role("option", /^Audit dependencies/), say: "The reason sits beside the line" },
  ];
}

const paletteModCommands = walk("palette-mod-commands", steps());

export { paletteModCommands as "palette-mod-commands" };
