// The title row at rest, its one menu with the account and Settings, and Settings opening. Over `title-bar-menu`.

import { button, dialog, inside, place, walk } from "../walk";

const MENU = dialog("Menu");

const titleBarMenu = walk("title-bar-menu", [
  { look: button("Menu", { exact: true }), say: "The row at rest: search, plan meters, Helm, and one menu" },
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { look: inside(MENU, button("Settings", { exact: true })), say: "The account and Settings" },
  { press: inside(MENU, button("Settings", { exact: true })), say: "Settings" },
  { press: place("Cockpit"), say: "Back on the Cockpit" },
]);

export { titleBarMenu as "title-bar-menu" };
