// A picker over many repositories: type to narrow it, and the next time the one picked leads under
// Recent. Told through the Autopilot card's picker, the one the deck draws while no repository is
// chosen, which lists the ones not set up as disabled. The left column's picker reads the Recent list
// when the Cockpit is drawn, so it is shown after a trip to Jobs and back.

import { button, place, role, text, walk } from "../walk";

export const findingOneRepositoryOfMany = walk("many-repositories", [
  { press: button("Repository", { exact: true }), say: "Twenty-one repositories, so the menu opens on a filter field" },
  { type: "arm", into: role("searchbox", "Find a repository"), say: "Typing narrows it to what matches" },
  { look: role("menuitem", "armada", { exact: true }), say: "Nothing else is left to read past but a repository not set up, which cannot be picked" },
  { press: role("menuitem", "armada", { exact: true }), say: "Picking one scopes Bridge to it" },
  { press: place("Jobs"), say: "To Jobs" },
  { press: place("Cockpit"), say: "And back on the Cockpit" },
  { press: button("Repository", { exact: true }), say: "The left column's picker, open again" },
  { look: text("Recent"), say: "The repository just picked leads, under Recent" },
]);
