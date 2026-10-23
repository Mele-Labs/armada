// What a taught Logbook entry also changes, for the line under its text: Bridge and Fleet, as the lesson it became reached them.
// Forget on the entry forgets the lesson, and with it every line here.

import type { ReactNode } from "react";
import { AlsoChanges } from "@armada/components";
import type { LogbookEntry } from "@armada/protocol";

import { askToOpenMod } from "./open-mod";
import { DOES, lessonsNow, SCOPE, taughtId } from "./teach";

export function alsoChanges(entry: LogbookEntry): ReactNode {
  const lesson = lessonsNow().find((one) => taughtId(one) === entry.id);
  if (lesson === undefined) return undefined;
  const { bridge, fleet, mod } = lesson.answer;
  const kept = mod === undefined ? {} : { mod: { name: mod, onOpen: () => askToOpenMod(mod) } };
  const lines = [
    ...(bridge === undefined ? [] : [{ part: "Bridge" as const, text: `${bridge.name}s link to ${lesson.reach === "all" ? "each repository's" : "this repository's"} site`, ...kept }]),
    ...(fleet === undefined ? [] : [{ part: "Fleet" as const, text: `${fleet.name.charAt(0).toLowerCase()}${fleet.name.slice(1)}, ${DOES[fleet.does]} ${SCOPE[fleet.scope]}`, ...kept }]),
  ];
  return lines.length === 0 ? undefined : <AlsoChanges lines={lines} />;
}
