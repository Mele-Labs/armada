// A Kit item whose fix is a command a Drone was refused (the owner, 5 Oct
// 2026): the button reads Update Kit, the card reads Updated Kit with the
// command, and the Kit page lists it as a retro item's, with Remove. `retro.test.tsx`
// and `kit.test.tsx` hold the claims.

import { button, inside, region, role, text, walk } from "../walk";

const GREP = role("listitem", "A Drone had to wait for grep to be allowed");
const ALLOWLIST = region("Allowlist");

export const updatingKitFromARetro = walk("retro/lessons", [
  { press: button("Retros", { exact: true }), say: "Retros, under Work" },
  { look: inside(GREP, button("Update Kit", { exact: true })), say: "A Kit item with a command to allow" },
  { hover: inside(GREP, button("Update Kit", { exact: true })), say: "What Update Kit does" },
  { press: inside(GREP, button("Update Kit", { exact: true })), say: "Update Kit adds the command" },
  { look: inside(GREP, text(/^Updated Kit$/)), say: "The card says Updated Kit" },
  { look: inside(GREP, text(/^grep$/)), say: "And the command it added" },
  { press: button("Kit", { exact: true }), say: "Kit" },
  { look: inside(ALLOWLIST, text(/^grep$/)), say: "The command, in the allowlist" },
  { look: inside(ALLOWLIST, text(/^Retro item$/)), say: "Where it came from" },
  { look: inside(ALLOWLIST, button("Remove grep")), say: "Remove takes it out" },
]);
