// What a Job costs in tokens: a switch on each skill in Kit › Skills for whether Jobs get it, three Machine settings
// under Fleet › Token use, and the connectors switch under Fleet › Repository. A design review before Fleet serves
// any of them. Over `token-usage-settings`.

import { button, inside, place, region, role, systemPage, walk } from "../walk";

const tokenUsageSettings = walk("token-usage-settings", [
  {
    press: place("System"),
    say: "System",
  },
  {
    press: systemPage("Skills"),
    say: "Skills, with a switch on each for whether Jobs get it",
  },
  {
    look: role("switch", "Jobs get gitnexus-cli"),
    say: "gitnexus-cli is off",
  },
  {
    press: role("switch", "Jobs get humanizer"),
    say: "humanizer off: Jobs start without it, so it costs nothing per request",
  },
  {
    look: inside(region("Skills"), button(/^release-notes/)),
    say: "A repository's own skills and Armada's have no switch. Every Job gets them",
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
    say: "Repository, under Fleet",
  },
  {
    look: role("switch", /^Connectors/),
    say: "Connectors, off for this repository",
  },
]);

export { tokenUsageSettings as "token-usage-settings" };
