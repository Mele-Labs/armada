// The home machine, first run: one menu button carrying a mark, the account entry, the device code and
// the link it is typed at, what signing in set up, and the shared home on the Kit page. Over `onboarding-home`.

import { button, dialog, inside, role, text, toSystemPage, walk } from "../walk";

const MENU = dialog("Menu");
const CODE = role("group", "Device code", { exact: true });
const SET_UP = (row: string) => inside(role("listitem", row, { exact: true }), role("img", "Done", { exact: true }));

const onboardingHome = walk("onboarding-home", [
  { look: button("Menu", { exact: true }), say: "Signed out: the menu button carries a mark" },
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: inside(MENU, button("Sign in", { exact: true })), say: "The account page" },
  { look: role("region", "Turns on", { exact: true }), say: "Signed out: what signing in turns on, sets up and never does" },
  { press: button("Sign in with GitHub", { exact: true }), say: "Sign in" },
  { look: CODE, say: "The code" },
  { hover: role("img", "Waiting for GitHub", { exact: true }), say: "Fleet is waiting" },
  { press: button("Open github.com/login/device"), say: "The page the code is typed at" },
  { look: SET_UP("Shared home"), say: "Identity, store and shared home, done" },
  ...toSystemPage("Kit", "Kit"),
  { look: text("nickmele/armada-home"), say: "The shared home, with Sync beside it" },
]);

export { onboardingHome as "onboarding-home" };
