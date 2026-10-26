// Settings → Phone walks the owner to a way in of his own: a Mac with no Tailscale installs it and signs
// in, turns Funnel on and allows it on the tailnet's page, then pairs (`phone-way-in`). Where
// `pocket-address` names a tunnel of his own, the steps are one line (`phone-way-in-named`).

import { button, inside, region, role, text, toSystemPage, walk } from "../walk";

const WAY_IN = region("Way in", { exact: true });
const STEP = (name: string) => role("listitem", name, { exact: true });
const DONE = (name: string) => inside(STEP(name), role("img", "Done", { exact: true }));

export const phoneWayIn = walk("phone/set-up", [
  ...toSystemPage("Phone", "Settings, Phone, on a Mac with nothing set up"),
  { look: WAY_IN, say: "Access is on: the way in, a row a step, before anything can pair" },
  { hover: inside(STEP("Install Tailscale on this Mac"), role("img", "Not done yet", { exact: true })), say: "A step not done yet" },
  { press: button("Get Tailscale", { exact: true }), say: "Get Tailscale opens its download" },
  { look: DONE("Sign in to Tailscale"), say: "Installed and signed in: both rows done, read again by themselves" },
  { look: inside(STEP("Turn on Funnel"), text(/^Funnel puts this Mac/)), say: "Funnel, and what it does" },
  { press: button("Turn on Funnel", { exact: true }), say: "Turn on Funnel" },
  { look: inside(STEP("Turn on Funnel"), text(/^Your tailnet has not allowed Funnel/)), say: "The tailnet has not allowed it yet" },
  { press: button("Allow Funnel", { exact: true }), say: "Allow Funnel opens Tailscale's page" },
  { look: DONE("Turn on Funnel"), say: "Funnel on, without another press" },
  { look: inside(STEP("Add Armada to the iPhone"), text(/^In Safari on the iPhone, open https:\/\/studio/)), say: "The phone's step names the address" },
  { press: button("Copy address", { exact: true }), say: "Copy address" },
  { press: button("Pair a phone", { exact: true }), say: "Pair a phone is there now" },
  { press: button("Confirm", { exact: true }), say: "The phone claims the code: Confirm" },
  { look: text(/through Tailscale Funnel\.$/), say: "Done: the steps fold to one line" },
]);

export const phoneWayInNamed = walk("phone/named", [
  ...toSystemPage("Phone", "Settings, Phone, where pocket-address names the way in"),
  { look: text(/the address pocket-address names\.$/), say: "One line, and no Tailscale steps" },
  { look: button("Pair a phone", { exact: true }), say: "Pairing straight away" },
]);
