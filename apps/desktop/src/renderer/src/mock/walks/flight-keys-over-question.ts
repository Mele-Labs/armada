// The flight controls wear ⌥⇧ and a number, so a bare number only ever answers the question. Over `flight-keys-over-question`.
// Keys: 1 picks an answer, ⌥⇧1 presses Update.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");
const controls = region("Flight controls");

export const flightKeysOverQuestion = walk("flight-keys-over-question", [
  { press: deckRow("Retry the slot lease"), say: "The Job is open, and its question is in Needs you" },
  { hover: inside(controls, button("Ask for update")), say: "The tile wears ⌥⇧1: the corner shows the number, and the tooltip spells the keys. Say if the corner should show ⌥⇧ too" },
  { key: "1", on: inside(needs, role("radio", "Carry on in a new Job", { exact: true })), say: "1 picks the first answer and presses no tile" },
]);
