// The weekly window nearing its end, and the default model offered a step down until it resets. Behind a
// source the way Sleep's state and the plan's windows are, so a Bridge with none draws nothing: no bell on
// the meter, no notification. Only the mock has one (`mock/downshift.ts`); Fleet serves none yet.

import { createContext, useContext, useSyncExternalStore } from "react";
import { Toast } from "@armada/components";
import type { LimitOffer, PlanUsage } from "@armada/components";

/** Where the offer stands. Absent: none, or the owner has settled it for this window. */
export type Downshift =
  /** The weekly window is at or over the line, and the default, `was`, is not yet changed. */
  | { phase: "offered"; percent: number; was: string; to: string }
  /** The default is `to` until `until`, the window's reset; `was` is what it goes back to. */
  | { phase: "shifted"; was: string; to: string; until: string }
  /** The window has reset and the default is still `to`. */
  | { phase: "reset"; was: string; to: string };

export type DownshiftSource = {
  get(): Downshift | undefined;
  subscribe(on: () => void): () => void;
  /** Default to the lower model until the window resets. */
  accept(): void;
  /** Back to the model it was. */
  restore(): void;
  /** Leave the lower model as the default. */
  keep(): void;
  /** The owner sets it by hand: nothing more is offered in this window. */
  settle(): void;
};

const DownshiftContext = createContext<DownshiftSource | null>(null);
export const DownshiftSourceProvider = DownshiftContext.Provider;

const NO_STATE = { get: () => undefined, subscribe: () => () => undefined };

/** A moment as the card says it: the time alone today, with its weekday after that. */
export function clockOf(iso: string, now = new Date()): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return "";
  const time = at.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" });
  return at.toDateString() === now.toDateString() ? time : `${at.toLocaleDateString("en-US", { weekday: "short" })} ${time}`;
}

function offerOf(state: Downshift, source: DownshiftSource, choose: () => void): LimitOffer {
  const choosing = { label: "Choose…", onPress: choose };
  if (state.phase === "offered") {
    return {
      attention: `Default to ${state.to} offered`,
      facts: [["Default model", state.was]],
      acts: [{ label: `Default to ${state.to}`, onPress: source.accept, primary: true, stays: true }, choosing],
    };
  }
  if (state.phase === "shifted") return { facts: [["Default model", `${state.to} until ${clockOf(state.until)}`]] };
  return {
    attention: `Back to ${state.was} offered`,
    facts: [["Default model", state.to]],
    acts: [{ label: `Back to ${state.was}`, onPress: source.restore, primary: true }, { label: `Keep ${state.to}`, onPress: source.keep }, choosing],
  };
}

/** The weekly row with the offer standing on it, where there is one. `choose` opens the Default model setting. */
export function useDownshiftOffer(usage: PlanUsage[] | undefined, choose: () => void): PlanUsage[] | undefined {
  const source = useContext(DownshiftContext);
  const state = useSyncExternalStore(source?.subscribe ?? NO_STATE.subscribe, source?.get ?? NO_STATE.get);
  if (usage === undefined || source === null || state === undefined) return usage;
  const offer = offerOf(state, source, () => {
    source.settle();
    choose();
  });
  return usage.map((one) => (one.kind === "seven_day" ? { ...one, offer } : one));
}

/**
 * The notification, while the offer or the restore stands. **One act, which is the card's primary**, since
 * a toast is never a second decision; pressing the sentence goes to the setting. It leaves when the state moves.
 */
export function DownshiftAlert({ choose }: { choose: () => void }) {
  const source = useContext(DownshiftContext);
  const state = useSyncExternalStore(source?.subscribe ?? NO_STATE.subscribe, source?.get ?? NO_STATE.get);
  if (source === null || state === undefined) return null;
  if (state.phase === "offered") {
    return (
      <Toast
        onPress={() => {
          source.settle();
          choose();
        }}
        actionLabel={`Default to ${state.to}`}
        onAction={source.accept}
      >
        {`Weekly limit ${Math.round(state.percent)}% used`}
      </Toast>
    );
  }
  if (state.phase === "reset") {
    return (
      <Toast actionLabel={`Back to ${state.was}`} onAction={source.restore}>
        Weekly limit reset
      </Toast>
    );
  }
  return null;
}
