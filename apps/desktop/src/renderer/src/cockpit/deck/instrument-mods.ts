// An instrument a mod supplies: a face painted on a bay of the console and a panel over the field when it is pressed. Nothing is on the wire yet, so a scenario hands the list in.
// The Job that builds one lives on its bay. While a bay shows it, it has no row (`listOf`).

import { useSyncExternalStore, type ComponentType } from "react";
import type { LucideIcon } from "lucide-react";

export type FaceInk = { run: string; ask: string; bad: string; good: string; fg: string; muted: string; subtle: string; off: string };
/** Draws a screen `w` by `h` in the facet's true proportions, so a circle stays a circle. */
export type FacePaint = (draw: CanvasRenderingContext2D, at: { w: number; h: number; t: number; ink: FaceInk }) => void;

export type InstrumentMod = {
  /** The mod's folder. */
  name: string;
  label: string;
  code: string;
  Icon: LucideIcon;
  paint: FacePaint;
  /** What stands in the panel the press opens. Absent while it is being built. */
  Panel?: ComponentType;
  /** The Mod Job that made it, which its bay's card is. */
  job?: string;
  /** Why it is not drawn: its socket is empty and says so, and its seat is kept. */
  fault?: string;
};
export type Made = { stub: InstrumentMod; built: Promise<InstrumentMod> };

type Held = { list: readonly InstrumentMod[]; make: ((said: string) => Made) | undefined; shown: ReadonlySet<string> };
let held: Held = { list: [], make: undefined, shown: new Set() };
const heard = new Set<() => void>();
const put = (next: Held): void => {
  held = next;
  heard.forEach((one) => one());
};
const useHeld = <T,>(read: (now: Held) => T): T =>
  useSyncExternalStore(
    (hear) => (heard.add(hear), () => void heard.delete(hear)),
    () => read(held),
  );

export const installInstruments = (list: readonly InstrumentMod[], make: Held["make"]): void => put({ list, make, shown: new Set() });
export const useInstruments = (): readonly InstrumentMod[] => useHeld((now) => now.list);

/** A face that threw: its socket stands empty with the reason. */
export const faultInstrument = (name: string, why: string): void => put({ ...held, list: held.list.map((one) => (one.name === name && one.fault === undefined ? { ...one, fault: why } : one)) });

/** Starts the Mod Job: the instrument is listed at once, being built, and is itself once the Job has made it. Answers its name. */
export function makeInstrument(said: string): string | undefined {
  if (held.make === undefined) return undefined;
  const { stub, built } = held.make(said);
  put({ ...held, list: [...held.list, stub] });
  void built.then((made) => put({ ...held, list: held.list.map((one) => (one.name === stub.name ? made : one)) }));
  return stub.name;
}

/** A bay is showing this Job, until the answer is called. */
export function bayShows(job: string): () => void {
  put({ ...held, shown: new Set([...held.shown, job]) });
  return () => put({ ...held, shown: new Set([...held.shown].filter((one) => one !== job)) });
}
/** The Jobs a bay is showing now. */
export const useBaysShowing = (): ReadonlySet<string> => useHeld((now) => now.shown);

/** A face while the Job has yet to write one: a bar going across. */
export const buildingFace: FacePaint = (draw, { w, h, t, ink }) => {
  const bar = w * 0.22;
  draw.fillStyle = ink.off;
  draw.fillRect(8, h / 2 - 2, w - 16, 4);
  draw.fillStyle = ink.run;
  draw.fillRect(8 + ((t * 0.5) % 1) * (w - 16 - bar), h / 2 - 2, bar, 4);
};
