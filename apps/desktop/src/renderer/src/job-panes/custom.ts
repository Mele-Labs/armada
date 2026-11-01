// A Custom tab: described to a Mod Job, built as a mod of kind `tab`, then drawn as the tab it made. What a tab is at is kept here by its id, not in the arrangement:
// a tab being built is no change of layout. Fleet's mod list says when the Job has written the mod, so a tab built in the background still turns over.

import { useEffect, useSyncExternalStore } from "react";

import { useModsMade } from "../mods-made";

export type CustomTab = { stage: "compose" | "building" | "built"; /** The Mod Job that is building it, or built it last. */ job?: string; /** The mod it draws, by its folder's name. */ mod?: string; /** The composer is open over the built view, for a change. */ editing?: boolean };

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
export const startBuilding = (id: string, job: string) => put(id, { ...peekCustom(id), stage: "building", job, editing: false });
/** A tab added by the name of a mod that is built already. */
export const seedBuilt = (id: string, mod: string) => put(id, { stage: "built", mod });
export const setEditing = (id: string, editing: boolean) => put(id, { ...peekCustom(id), editing });
const finished = (id: string, mod: string) => put(id, { ...peekCustom(id), stage: "built", mod, editing: false });

/** Re-renders on any change of a Custom tab, for what reads a label or a glyph. */
export const useCustomTick = (): void => void useSyncExternalStore(subscribe, () => version);

/** The tab's state. Turns a building tab over once Fleet's list holds a `tab` mod its Job made, wherever the tab is drawn. */
export function useCustom(id: string, custom: boolean): CustomTab {
  useCustomTick();
  const now = peekCustom(id);
  const { mods } = useModsMade(custom && now.stage === "building" ? (now.job ?? "") : "");
  const made = mods.find((one) => one.kind === "tab")?.name;
  useEffect(() => {
    if (made !== undefined) finished(id, made);
  }, [id, made]);
  return now;
}
