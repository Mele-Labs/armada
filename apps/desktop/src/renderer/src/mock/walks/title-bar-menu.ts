// The title row at rest, its one menu with the account, Sleep and Settings, Sleep turned on and the moon
// on the menu button, and Settings opening. Over `title-bar-menu`.

import { button, dialog, inside, place, walk } from "../walk";

const MENU = dialog("Menu");

const titleBarMenu = walk("title-bar-menu", [
  { look: button("Menu", { exact: true }), say: "The row at rest: search, plan meters, Helm, and one menu" },
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: inside(MENU, button("Sleep mode", { exact: true })), say: "Account, Sleep and Settings: turn Sleep on" },
  { look: button("Menu", { exact: true }), say: "The moon rides on the menu button" },
  { press: inside(MENU, button("Settings", { exact: true })), say: "Settings" },
  { press: place("Cockpit"), say: "Back on the Cockpit" },
]);

export { titleBarMenu as "title-bar-menu" };
