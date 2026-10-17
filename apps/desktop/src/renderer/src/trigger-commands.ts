// Bridge's calls for Triggers, kept out of `commands.ts` for its length.

import type { ReadingTrigger, RemovingTrigger, SavingTrigger } from "../../shared/triggers";

/** Triggers: what the picked repository runs, one as its file holds it, a save and a removal. */
export const readTriggers = () => window.armada.readTriggers();
export const readTrigger = (reading: ReadingTrigger) => window.armada.readTrigger(reading);
export const saveTrigger = (saving: SavingTrigger) => window.armada.saveTrigger(saving);
export const removeTrigger = (removing: RemovingTrigger) => window.armada.removeTrigger(removing);
