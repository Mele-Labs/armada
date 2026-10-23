// A Kit item whose fix is a command an agent was refused (the owner, 5 Oct
// 2026): the button reads Update Kit, the card reads Updated Kit with the
// command, and the Kit page lists it as a retro item's, with Remove. `retro-page.test.tsx`
// and `kit.test.tsx` hold the claims.

import { button, inside, region, role, text, toSystemPage, walk } from "../walk";

const GREP = role("listitem", "grep had to be allowed on a check log");
const ALLOWLIST = region("Allowlist");

export const updatingKitFromARetro = walk("retro/kit-command", [
  ...toSystemPage("Retros", "The Logbook, with Armada fixes under the entries"),
  { look: inside(GREP, button("Update Kit", { exact: true })), say: "A Kit item with a command to allow" },
  { hover: inside(GREP, button("Update Kit", { exact: true })), say: "What Update Kit does" },
  { press: inside(GREP, button("Update Kit", { exact: true })), say: "Update Kit adds the command" },
  { look: inside(GREP, text(/^Updated Kit$/)), say: "The card says Updated Kit" },
  { look: inside(GREP, text(/^grep$/)), say: "And the command it added" },
  ...toSystemPage("Allowlist", "Allowlist"),
  { look: inside(ALLOWLIST, text(/^grep$/)), say: "The command, in the allowlist" },
  { look: inside(ALLOWLIST, text(/^Retro item$/)), say: "Where it came from" },
  { press: inside(ALLOWLIST, button(/^grep( |$)/)), say: "The command" },
  { look: button("Remove grep"), say: "Remove takes it out" },
]);
