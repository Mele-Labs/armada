// What a Job costs in tokens, set per repository where it differs by one: a skill's detail in Kit › Skills lists the
// repositories whose Jobs get it, Fleet › Token use holds three Machine settings, and Fleet › Repository has a
// connectors switch per repository. A design review before Fleet serves any of them. Over `token-usage-settings`.

import { button, inside, place, region, role, systemPage, walk } from "../walk";

const tokenUsageSettings = walk("token-usage-settings", [
  {
    press: place("System"),
    say: "System",
  },
  {
    press: systemPage("Skills"),
    say: "Skills",
  },
  {
    press: inside(region("Skills"), button(/^gitnexus-cli/)),
    say: "gitnexus-cli, with the repositories whose Jobs get it",
  },
  {
    press: role("switch", "Jobs in storefront get gitnexus-cli"),
    say: "Off for storefront: its Jobs start without gitnexus-cli, and the other repositories keep it",
  },
  {
    press: inside(region("Skills"), button(/^release-notes/)),
    say: "A repository's own skill has no switches. Every Job gets it",
  },
  {
    press: systemPage("Token use"),
    say: "Token use, under Fleet",
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
    say: "Connectors, one switch per repository. armada is off from its armada.yml",
  },
  {
    press: role("switch", "Connectors in ledger"),
    say: "Off for ledger: its Jobs start without your account's connectors",
  },
]);

export { tokenUsageSettings as "token-usage-settings" };
