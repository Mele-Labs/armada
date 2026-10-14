// The work machine, first run: the work account signs in alone, then the personal login is added from
// the account chip and What syncs opens with every kind off. Turning one on fills it in with its
// items and their scopes, and the chip then opens both profiles. Over `onboarding-work`.

import { button, dialog, inside, role, walk } from "../walk";

const SYNCS = dialog("What syncs");

const onboardingWork = walk("onboarding-work", [
  { press: button("Sign in with GitHub", { exact: true }), say: "Sign in with the work account" },
  { press: button("Open github.com/login/device"), say: "The code is typed at GitHub" },
  { press: button("Kit", { exact: true }), say: "Its own identity, store and shared home, and its own Kit" },
  { press: button("Account mele-work", { exact: true }), say: "The account chip" },
  { press: button("Add profile", { exact: true }), say: "Add profile" },
  { press: button("Open github.com/login/device"), say: "The personal login signs in" },
  { press: inside(SYNCS, role("switch", "Skills", { exact: true })), say: "Every kind is off" },
  { hover: inside(SYNCS, button("Private, recipe-scaler")), say: "Skills filled in, a scope on each item" },
  { press: button("Account mele-work", { exact: true }), say: "The chip opens both profiles" },
]);

export { onboardingWork as "onboarding-work" };
