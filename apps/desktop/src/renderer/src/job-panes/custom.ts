// A Custom tab: described to a Mod Job, built as a mod of kind `tab`, then drawn as the tab it made. What a tab is at is kept here by its id, not in the arrangement:
// a tab being built is no change of layout. Fleet's mod list says when the Job has written the mod, so a tab built in the background still turns over.

import { useEffect, useSyncExternalStore } from "react";

import { useModList } from "../mod-list";

export type CustomTab = { stage: "compose" | "building" | "built"; /** The Mod Job that is building it, or built it last. A tab added by its mod's name has its mod's `made_by`. */ job?: string; /** The mod it draws, by its folder's name. */ mod?: string; /** The mod's glyph, by its name in the icon registry. */ glyph?: string; };

const held = new Map<string, CustomTab>();
const listeners = new Set<() => void>();
let version = 0;
const COMPOSE: CustomTab = { stage: "compose" };

const put = (id: string, next: CustomTab) => {
  held.set(id, next);
  version += 1;
  listeners.forEach((on) => on());
};
const subscribe = (on: () => void) => {
  listeners.add(on);
  return () => void listeners.delete(on);
};

export const peekCustom = (id: string): CustomTab => held.get(id) ?? COMPOSE;

/** A mod's folder name as the tab reads: `review-queue` is Review queue. */
export const tabTitle = (name: string): string => {
  const words = name.replace(/[-_]+/g, " ").trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
};

/** The tab's words once it has a mod, absent before. */
export const customLabel = (id: string): string | undefined => {
  const one = held.get(id);
  return one?.mod === undefined ? undefined : tabTitle(one.mod);
};

/** The description went to a Mod Job: the tab is its building state. */
export const startBuilding = (id: string, job: string) => put(id, { ...peekCustom(id), stage: "building", job });
/** A tab added by the name of a mod that is built already. */
export const seedBuilt = (id: string, mod: string) => put(id, { stage: "built", mod });
const finished = (id: string, mod: string) => put(id, { ...peekCustom(id), stage: "built", mod });
/** A built tab another Mod Job is now changing: its label and hover card follow that Job. */
export const followJob = (id: string, job: string) => put(id, { ...peekCustom(id), job });

/** Re-renders on any change of a Custom tab, for what reads a label or a glyph. */
export const useCustomTick = (): void => void useSyncExternalStore(subscribe, () => version);

/** The tab's state. Turns a building tab over once Fleet's list holds a `tab` mod its Job made, wherever the tab is drawn. */
export function useCustom(id: string, custom: boolean): CustomTab {
  useCustomTick();
  const now = peekCustom(id);
  const listed = useModList();
  const made = custom && now.stage === "building" ? listed.find((one) => one.kind === "tab" && one.made_by === now.job)?.name : undefined;
  useEffect(() => {
    if (made !== undefined) finished(id, made);
  }, [id, made]);
  const row = now.mod === undefined ? undefined : listed.find((one) => one.name === now.mod);
  const job = now.job ?? row?.made_by;
  return { ...now, ...(job === undefined ? {} : { job }), ...(row?.glyph === undefined ? {} : { glyph: row.glyph }) };
}
