// Picking a Workspace heading in the repository picker scopes the window to its repositories (armada-cwj). Opens on All repositories with six served.

import { button, deckRow, place, role, text, walk } from "../walk";

export const workspaceScope = walk(
  "workspace-scope",
  [
    { press: button("Repository"), say: "The picker, with my-store and platform as headings" },
    { press: role("menuitem", "my-store", { exact: true }), say: "my-store, picked: the window shows store-web and store-api" },
    { look: deckRow("Round the cart totals"), say: "store-web's Jobs, banded as All bands them" },
    { look: deckRow("Add the gift card endpoint"), say: "store-api's Job beside them. scratch and store-mobile are not drawn" },
    { press: place("Studios"), say: "One panel for each repository in my-store" },
    { look: text("Gift card flow"), say: "store-web's Studios" },
    { look: text("Gift card endpoint"), say: "store-api's Studio" },
  ],
  { width: 1600, height: 1000 },
);

export { workspaceScope as "workspace-scope" };
