// What a taught Logbook entry also changes, for the line under its text: a line for each mod the lesson made, Words or Fleet.
// Forget on the entry forgets the lesson, and with it every line here.

import type { ReactNode } from "react";
import { AlsoChanges } from "@armada/components";
import type { LogbookEntry } from "@armada/protocol";

import { askToOpenMod } from "./open-mod";
import { DOES, lessonsNow, SCOPE, taughtId } from "./teach";

export function alsoChanges(entry: LogbookEntry): ReactNode {
  const lesson = lessonsNow().find((one) => taughtId(one) === entry.id);
  if (lesson === undefined) return undefined;
  const { bridge, fleet } = lesson.answer;
  const lines = [
    ...(bridge === undefined ? [] : [{ part: "Words" as const, text: `${bridge.name}s link to ${lesson.reach === "all" ? "each repository's" : "this repository's"} site`, mod: { name: bridge.mod, onOpen: () => askToOpenMod(bridge.mod, "words") } }]),
    ...(fleet === undefined ? [] : [{ part: "Fleet" as const, text: `${fleet.name.charAt(0).toLowerCase()}${fleet.name.slice(1)}, ${DOES[fleet.does]} ${SCOPE[fleet.scope]}`, mod: { name: fleet.mod, onOpen: () => askToOpenMod(fleet.mod, "fleet") } }]),
  ];
  return lines.length === 0 ? undefined : <AlsoChanges lines={lines} />;
}
