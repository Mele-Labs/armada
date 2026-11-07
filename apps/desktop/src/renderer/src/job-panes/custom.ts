// A Custom tab: described to a Mod Job, built as a mod of kind `tab`, then drawn as the tab it made. What a tab is at is kept here by its Job and its id, not in the arrangement:
// a tab being built is no change of layout. Fleet's mod list says when the Job has written the mod, so a tab built in the background still turns over.

import { useEffect, useSyncExternalStore } from "react";

import { forgetKept } from "@armada/components";

import { useModList } from "../mod-list";

export type CustomTab = {
  stage:
    | "compose"
    | "building"
    | "built" /** The Mod Job that is building it, or built it last. A tab added by its mod's name has its mod's `made_by`. */;
  job?: string /** The mod it draws, by its folder's name. */;
  mod?: string /** The mod's glyph, by its name in the icon registry. */;
  glyph?: string;
};

/** Tab ids are minted per window, so two Jobs can hold the same one: a tab's state is the Job's, then the tab's. */
const held = new Map<string, Map<string, CustomTab>>();
const listeners = new Set<() => void>();
let version = 0;
const COMPOSE: CustomTab = {
  stage: "compose",
};

const put = (jobId: string, id: string, next: CustomTab) => {
  const tabs = held.get(jobId) ?? new Map<string, CustomTab>();
  tabs.set(id, next);
  held.set(jobId, tabs);
  version += 1;
  listeners.forEach((on) => on());
};
const subscribe = (on: () => void) => {
  listeners.add(on);
  return () => void listeners.delete(on);
};

export const peekCustom = (jobId: string, id: string): CustomTab =>
  held.get(jobId)?.get(id) ?? COMPOSE;

/** The key the tab's unsent description is kept under, apart for each Job. */
export const customDraftKey = (jobId: string, tabId: string): string =>
  `custom:${jobId}:${tabId}`;

const KEPT = "armada:kept:";
const keptNames = (): string[] => {
  try {
    const store = window.localStorage;
    return Array.from({ length: store.length }, (_, at) => store.key(at) ?? "")
      .filter((key) => key.startsWith(KEPT))
      .map((key) => key.slice(KEPT.length));
  } catch {
    return [];
  }
};

/** The Job is gone: its tabs' state and their kept descriptions go with it. */
export const forgetCustom = (jobId: string): void => {
  held.delete(jobId);
  const drafts = `session:custom:${jobId}:`;
  keptNames()
    .filter((name) => name.startsWith(drafts))
    .forEach(forgetKept);
  version += 1;
  listeners.forEach((on) => on());
};

/** Descriptions kept before they named their Job, as `session:custom:<tab id>`. A tab id holds no `:`, so a name with one is the new shape. */
export const forgetOldCustomDrafts = (): void => {
  const old = "session:custom:";
  keptNames()
    .filter((name) => name.startsWith(old) && !name.slice(old.length).includes(":"))
    .forEach(forgetKept);
};

/** A mod's folder name as the tab reads: `review-queue` is Review queue. */
export const tabTitle = (name: string): string => {
  const words = name.replace(/[-_]+/g, " ").trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
};

/** The tab's words once it has a mod, absent before. */
export const customLabel = (jobId: string, id: string): string | undefined => {
  const one = held.get(jobId)?.get(id);
  return one?.mod === undefined ? undefined : tabTitle(one.mod);
};

/** The description went to a Mod Job: the tab is its building state. */
export const startBuilding = (jobId: string, id: string, job: string) =>
  put(jobId, id, {
    ...peekCustom(jobId, id),
    stage: "building",
    job,
  });
/** A tab added by the name of a mod that is built already. */
export const seedBuilt = (jobId: string, id: string, mod: string) =>
  put(jobId, id, {
    stage: "built",
    mod,
  });
const finished = (jobId: string, id: string, mod: string) =>
  put(jobId, id, {
    ...peekCustom(jobId, id),
    stage: "built",
    mod,
  });
/** A built tab another Mod Job is now changing: its label and hover card follow that Job. */
export const followJob = (jobId: string, id: string, job: string) =>
  put(jobId, id, {
    ...peekCustom(jobId, id),
    job,
  });

/** Re-renders on any change of a Custom tab, for what reads a label or a glyph. */
export const useCustomTick = (): void => void useSyncExternalStore(subscribe, () => version);

/** The tab's state. Turns a building tab over once Fleet's list holds a `tab` mod its Job made, wherever the tab is drawn. */
export function useCustom(jobId: string, id: string, custom: boolean): CustomTab {
  useCustomTick();
  const now = peekCustom(jobId, id);
  const listed = useModList();
  const made =
    custom && now.stage === "building"
      ? listed.find((one) => one.kind === "tab" && one.made_by === now.job)?.name
      : undefined;
  useEffect(() => {
    if (made !== undefined) finished(jobId, id, made);
  }, [
    jobId,
    id,
    made,
  ]);
  const row = now.mod === undefined ? undefined : listed.find((one) => one.name === now.mod);
  const job = now.job ?? row?.made_by;
  return {
    ...now,
    ...(job === undefined
      ? {}
      : {
          job,
        }),
    ...(row?.glyph === undefined
      ? {}
      : {
          glyph: row.glyph,
        }),
  };
}
