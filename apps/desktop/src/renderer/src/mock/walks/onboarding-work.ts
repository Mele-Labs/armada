// The work machine, first run: the work account signs in alone from the menu, then the personal login is
// added from the account entry and What syncs opens with every kind off. Turning one on fills it in with
// its items and their scopes. Over `onboarding-work`.

import { button, dialog, inside, role, toSystemPage, walk } from "../walk";

const MENU = dialog("Menu");
const SYNCS = dialog("What syncs");

const onboardingWork = walk("onboarding-work", [
  { press: button("Menu", { exact: true }), say: "Open the menu" },
  { press: inside(MENU, button("Sign in with GitHub", { exact: true })), say: "Sign in with the work account" },
  { press: button("Open github.com/login/device"), say: "The code is typed at GitHub" },
  ...toSystemPage("Kit", "Its own identity, store and shared home, and its own Kit"),
  { press: button("Menu", { exact: true }), say: "The menu again" },
  { press: inside(MENU, button("Account mele-work", { exact: true })), say: "The account" },
  { press: button("Add profile", { exact: true }), say: "Add profile" },
  { press: button("Open github.com/login/device"), say: "The personal login signs in" },
  { press: inside(SYNCS, role("switch", "Skills", { exact: true })), say: "Every kind is off" },
]);

export { onboardingWork as "onboarding-work" };
