// What a Job costs in tokens, as four Machine settings under Fleet › Token use, and the connectors switch under
// Fleet › Repository. A design review before Fleet serves any of them. Over `token-usage-settings`.

import { place, role, systemPage, walk } from "../walk";

const tokenUsageSettings = walk("token-usage-settings", [
  {
    press: place("System"),
    say: "System",
  },
  {
    press: systemPage("Token use"),
    say: "Token use, under Fleet",
  },
  {
    look: role("textbox", "Skills hidden from Jobs"),
    say: "Skills hidden from Jobs, one name per line",
  },
  {
    look: role("textbox", "Compaction window"),
    say: "Compaction window, in tokens",
  },
  {
    look: role("switch", /^Short cache for turn summaries/),
    say: "Short cache for turn summaries, on",
  },
  {
    look: role("switch", /^Thinking for turn summaries/),
    say: "Thinking for turn summaries, off",
  },
  {
    press: systemPage("Repository"),
    say: "Repository, under Fleet",
  },
  {
    look: role("switch", /^Connectors/),
    say: "Connectors, off for this repository",
  },
]);

export { tokenUsageSettings as "token-usage-settings" };
