// The home machine, first run: one menu button carrying a mark, the account entry, the device code and
// the link it is typed at, what signing in set up, and the shared home on the Kit page. Over `onboarding-home`.

import { button, dialog, inside, role, text, walk } from "../walk";

const MENU = dialog("Menu");
const SET_UP = (row: string) => inside(role("listitem", row, { exact: true }), role("img", "Done", { exact: true }));

const onboardingHome = walk("onboarding-home", [
  { look: button("Menu", { exact: true }), say: "Signed out: the menu button carries a mark" },
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: inside(MENU, button("Sign in", { exact: true })), say: "The account page" },
  { press: button("Sign in with GitHub", { exact: true }), say: "Sign in" },
  { look: text("WDJB-MJHT"), say: "The code" },
  { hover: role("img", "Waiting for GitHub", { exact: true }), say: "Fleet is waiting" },
  { press: button("Open github.com/login/device"), say: "The page the code is typed at" },
  { look: SET_UP("Shared home"), say: "Identity, store and shared home, done" },
  { press: button("Kit", { exact: true }), say: "Kit" },
  { look: text("nickmele/armada-home"), say: "The shared home, with Sync beside it" },
]);

export { onboardingHome as "onboarding-home" };
