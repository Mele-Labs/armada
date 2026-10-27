// A Job's watches kept tidy. Fleet sends readings that read the same back to back as one run, so QA health's nine identical readings are one row
// in the side panel with its count and its first and last times. The Watching block lists live watches only; Show ended draws the ended ones,
// each with Delete in a column every row keeps, and Delete takes one away. Over `watching`.

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";

const landing = region("Now", { exact: true });
const watchingBlock = inside(landing, region("Watching"));
const health = dialog("Watch QA health");
const readings = inside(health, role("list", "Readings"));

const watchingTidy = walk("watching", [
  { press: deckRow("Ship the rate limit to QA"), say: "A Job with one live watch and two that ended" },
  { look: watchingBlock, say: "Watching lists the live watch only" },
  { press: inside(watchingBlock, button("Watch QA health")), say: "QA health opens in the side panel" },
  { look: inside(readings, text("×9")), say: "Nine readings that read the same are one row: its glyph, what it said, ×9, and its first and last times" },
  { key: "Escape", on: health, say: "Esc closes it" },
  { press: inside(watchingBlock, role("switch", "Show ended")), say: "Show ended draws the two ended watches" },
  { look: watchingBlock, say: "Each ended watch has Delete in the last column, and the live one keeps that column empty" },
  { hover: inside(watchingBlock, button("Delete watch Build image")), say: "Delete, named on hover" },
  { press: inside(watchingBlock, button("Delete watch Build image")), say: "Delete takes Build image away" },
  { look: watchingBlock, say: "QA health and QA deploy are left" },
]);

export { watchingTidy };
