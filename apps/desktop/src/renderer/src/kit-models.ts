// Kit › Models: the models Claude Code offers, read through Fleet's `list_models`. Mock only until Fleet
// answers: the mock window provides a source (`mock/kit-models.ts`), and a window without one says the
// models are not read.

import { useSyncExternalStore } from "react";
import type { KitModelsRead, KitModelSlot } from "@armada/components";

export type KitModelsSource = {
  get: () => KitModelsRead;
  subscribe: (on: () => void) => () => void;
  /** Whether Jobs may use a model. */
  setOn: (name: string, on: boolean) => void;
  /** The model a kind of run starts on, or the effort. */
  choose: (slot: KitModelSlot, value: string) => void;
};

const NOT_READ: KitModelsRead = { read: "not_read", why: "Fleet does not list models yet" };

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

export function useKitModels(): { models: KitModelsRead; setOn: KitModelsSource["setOn"]; choose: KitModelsSource["choose"] } {
  const models = useSyncExternalStore(subscribe, () => provided?.get() ?? NOT_READ);
  return {
    models,
    setOn: (name, on) => provided?.setOn(name, on),
    choose: (slot, value) => provided?.choose(slot, value),
  };
}
