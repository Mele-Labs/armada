// What was taught is kept as a rules mod. From the Jira key and the notice teaching already saved: the Logbook entry names its mod and
// opens it in Mods, the mod holds the pattern and each repository's address with where it was found, and switching it off makes the
// keys plain again. The notice's mod sits in the same list. Over the `teach-mods` scenario, which starts with both teachings saved.

import { kit, toSessions } from "../sessions/walk-kit";
import { ARMADA_TITLE, KEY_MESSAGE } from "../sessions/teach";
import { button, deckRow, inside, place, region, role, text, walk } from "../walk";
import type { Step } from "../walk";

const KEY_WORDS = "STORE-1234";

function steps(): Step[] {
  const { thread } = kit(false);
  const entries = role("listbox", "Entries");
  const also = inside(region("Entry"), role("group", "Also changes"));
  const jira = role("group", "Rules of jira-keys");
  const keys = role("switch", "jira-keys");
  return [
    { press: place("Logbook"), say: "The Logbook page" },
    { press: inside(entries, text(/Tickets are Jira keys/)), say: "The Jira key entry" },
    { press: inside(also, button(/jira-keys/)), say: "Also changes names the mod that keeps it, and opens it in Mods" },
    { look: inside(jira, text("Found in pull request #2712")), say: "The pattern, and an address for each repository with where it was found" },
    { press: keys, say: "Switched off" },
    toSessions,
    { press: deckRow(ARMADA_TITLE), say: "The first Job" },
    { look: inside(thread, text(KEY_MESSAGE.slice(0, 40))), say: "The key is plain" },
    { press: place("System"), say: "Back in Mods" },
    { press: keys, say: "Switched on" },
    toSessions,
    { press: deckRow(ARMADA_TITLE), say: "The first Job" },
    { look: inside(thread, role("link", KEY_WORDS)), say: "The key links again" },
    { press: place("System"), say: "Mods again" },
    { key: "ArrowRight", on: role("button", "Rules of slot-behind-batched"), say: "The right arrow opens the other mod's rules" },
    { look: role("group", "Rules of slot-behind-batched"), say: "The notice, what Fleet does with it, and where" },
  ];
}

const teachMods = walk("teach-mods", steps());

export { teachMods as "teach-mods" };
