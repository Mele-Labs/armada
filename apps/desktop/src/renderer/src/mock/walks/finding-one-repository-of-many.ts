// A picker over many repositories: type to narrow it, Enter takes the first row left,
// and the next time the one picked leads under Recent.

import { button, role, text, walk } from "../walk";

export const findingOneRepositoryOfMany = walk("many-repositories", [
  { press: button("All repositories"), say: "Twenty-one repositories, so the menu opens on a filter field" },
  { type: "bill", into: role("searchbox", "Find a repository"), say: "Typing narrows it to what matches" },
  { look: role("menuitem", "billing-api"), say: "Nothing else is left to read past" },
  { press: role("menuitem", "billing-api"), say: "Picking one scopes Bridge to it" },
  { press: button("billing-api"), say: "Open the picker again" },
  { look: text("Recent"), say: "The repository just picked leads, under Recent" },
]);
