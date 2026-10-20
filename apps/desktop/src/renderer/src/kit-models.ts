// Kit › Models: the models the agent offers, read through Fleet's `list_models` and saved into settings.json.
// A mock window provides a source of its own (`mock/kit-models.ts`); a window without one is Fleet's.

import { useSyncExternalStore } from "react";
import type { KitModelsRead, KitModelSlot } from "@armada/components";
import type { ModelChoices, SettingsList } from "@armada/protocol";

import { readKitModels, saveForChoice, saveForSwitch } from "./kit-models-wire";

export type KitModelsSource = {
  get: () => KitModelsRead;
  subscribe: (on: () => void) => () => void;
  /** Whether Jobs may use a model. */
  setOn: (name: string, on: boolean) => void;
  /** The model a kind of run starts on, or the effort. */
  choose: (slot: KitModelSlot, value: string) => void;
};

let provided: KitModelsSource | undefined;
const asked = new Set<() => void>();

/** The window's source, or none. */
export function provideKitModels(source: KitModelsSource | undefined): void {
  provided = source;
  asked.forEach((on) => on());
}

const subscribe = (on: () => void) => {
  asked.add(on);
  const off = provided?.subscribe(on);
  return () => {
    asked.delete(on);
    off?.();
  };
};

type Held = { models: KitModelsRead; setOn: KitModelsSource["setOn"]; choose: KitModelsSource["choose"] };

/** The page's models: the window's own source where it provides one, else Fleet's `holds.models` and settings. */
export function useKitModels(held: ModelChoices | null, settings: SettingsList | null): Held {
  const source = useSyncExternalStore(subscribe, () => provided?.get());
  if (source !== undefined) {
    return { models: source, setOn: (name, on) => provided?.setOn(name, on), choose: (slot, value) => provided?.choose(slot, value) };
  }
  return {
    models: readKitModels(held, settings),
    setOn: (name, on) => {
      const changes = held === null ? null : saveForSwitch(held, settings, name, on);
      if (changes !== null) void window.armada.saveSettings(changes);
    },
    choose: (slot, value) => void window.armada.saveSettings(saveForChoice(slot, value)),
  };
}
