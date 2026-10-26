// The flight controls wear ⌥⇧ and a number, so a bare number only ever answers the question. Over `flight-keys-over-question`.
// Keys: 1 picks an answer, ⌥⇧1 presses Artifacts.

import { button, deckRow, inside, region, role, walk } from "../walk";

const needs = region("Needs you");

export const flightKeysOverQuestion = walk("flight-keys-over-question", [
  { press: deckRow("Retry the slot lease"), say: "The Job is open, and its question is in Needs you" },
  { hover: button("Artifacts", { exact: true }), say: "Artifacts wears ⌥⇧1, and the tooltip spells the keys" },
  { key: "1", on: inside(needs, role("radio", "Carry on in a new Job", { exact: true })), say: "1 picks the first answer and presses no tile" },
]);
