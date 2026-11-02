// An instrument a mod supplies: a face painted on a bay of the console and a panel over the field when it is pressed. Fleet's mod list names them, and `instrument-mount.tsx` keeps the
// component of each one seated mounted and says here what it drew. The Job that builds one lives on its bay. While a bay shows it, it has no row (`listOf`).

import { useMemo, useSyncExternalStore } from "react";
import type { LucideIcon } from "lucide-react";
import type { ModSummary } from "@armada/protocol";

import { refusal } from "../../annotate/send";
import { tabTitle } from "../../job-panes/custom";
import { instrumentGlyph } from "../../job-panes/tab-glyph";
import type { TabHost } from "../../job-panes/tab-module";
import { useModList } from "../../mod-list";
import { askToTell } from "../../tell";
import { skipsMods } from "../../theme-loader";

export type FaceInk = {
  run: string;
  ask: string;
  bad: string;
  good: string;
  fg: string;
  muted: string;
  subtle: string;
  off: string;
};
/** Draws a screen `w` by `h` in the facet's true proportions, so a circle stays a circle. */
export type FacePaint = (
  draw: CanvasRenderingContext2D,
  at: {
    w: number;
    h: number;
    t: number;
    ink: FaceInk;
  },
) => void;

/** What an instrument is handed: what a tab is, less the Job and the tab it has none of, and `face`, which registers what the bay draws. */
export type InstrumentHost = Omit<TabHost, "job" | "tab"> & {
  face: (
    paint: FacePaint,
    options?: {
      moves?: boolean;
    },
  ) => void;
};

/** What a bay draws. `rev` moves each time the mod asks to be drawn again, and a face that does not `moves` is drawn only then. */
export type Face = {
  paint: FacePaint;
  moves: boolean;
  rev: number;
};

export type InstrumentMod = {
  /** The mod's folder, or its seat while no mod is listed yet. */
  name: string;
  /** What a bay holds to hold this. */
  seat: `mod:${string}` | `job:${string}`;
  label: string;
  code: string;
  Icon: LucideIcon;
  face?: Face;
  /** The node its component renders into, which stands in the panel the press opens. Absent while nothing has loaded. */
  slot?: HTMLElement;
  /** The Mod Job that made it, which its bay's card is. */
  job?: string;
  /** Why it is not drawn: its socket is empty and says so, and its seat is kept. */
  fault?: string;
  /** The fault is his own switch. */
  off?: boolean;
};

const MOD = "mod:";
const JOB = "job:";

/** A stencil on the console, as the instruments have one: the first letters of a label. Decoration only. */
export const stencilOf = (label: string): string =>
  label
    .replace(/[^a-z0-9]/gi, "")
    .slice(0, 3)
    .toUpperCase();

// ---- What each mounted instrument has said ---------------------------------------------------------------

/** A mounted instrument: the load it is on, why that load is refused, and what it drew and threw, each with the load it came from, so the next load clears them. */
type Mount = {
  load: number;
  loaded: boolean;
  refused?: string;
  off: boolean;
  face?: Face & {
    load: number;
  };
  thrown?: {
    load: number;
    why: string;
  };
};
type Held = {
  mounts: ReadonlyMap<string, Mount>;
  shown: ReadonlySet<string>;
};
const EMPTY: Held = {
  mounts: new Map(),
  shown: new Set(),
};
let held = EMPTY;
const heard = new Set<() => void>();
const put = (next: Held): void => {
  held = next;
  heard.forEach((one) => one());
};
const useHeld = <T>(read: (now: Held) => T): T =>
  useSyncExternalStore(
    (hear) => (heard.add(hear), () => void heard.delete(hear)),
    () => read(held),
  );

const FRESH: Mount = {
  load: 0,
  loaded: false,
  off: false,
};
const putMount = (name: string, change: (was: Mount) => Mount): void => {
  const mounts = new Map(held.mounts);
  mounts.set(name, change(mounts.get(name) ?? FRESH));
  put({
    ...held,
    mounts,
  });
};

/** What `useTabModule` answered. Heard after the component's own first effects, so a face it registered at once stands, and the last load's stays until then and goes if there is none. */
export function mountLoaded(
  name: string,
  now: {
    load: number;
    loaded: boolean;
    refused: string | undefined;
    off: boolean;
  },
): void {
  putMount(name, (was) => {
    const { refused: _refused, face, thrown, ...kept } = was;
    return {
      ...kept,
      load: now.load,
      loaded: now.loaded,
      off: now.off,
      ...(now.refused === undefined
        ? {}
        : {
            refused: now.refused,
          }),
      ...(face?.load === now.load
        ? {
            face,
          }
        : {}),
      ...(thrown?.load === now.load
        ? {
            thrown,
          }
        : {}),
    };
  });
}

let revs = 0;
/** `host.face`: draw this. A paint that threw may have only lacked what it has now, so it is tried again. */
export function faceRegistered(name: string, load: number, paint: FacePaint, moves: boolean): void {
  revs += 1;
  const rev = revs;
  putMount(name, ({ thrown: _thrown, ...was }) => ({
    ...was,
    face: {
      paint,
      moves,
      rev,
      load,
    },
  }));
}

/** It threw as it rendered, on this load. */
export const renderThrew = (name: string, load: number, why: string): void =>
  putMount(name, (was) => ({
    ...was,
    thrown: {
      load,
      why,
    },
  }));

/** A face that threw: its socket stands empty with the reason. */
export function faultInstrument(name: string, why: string): void {
  const face = held.mounts.get(name)?.face;
  if (face === undefined || held.mounts.get(name)?.thrown?.load === face.load) return;
  renderThrew(name, face.load, why);
}

/** The instrument is no longer seated: it is drawn nowhere and holds nothing. */
export function forgetMount(name: string): void {
  if (!held.mounts.has(name)) return;
  const mounts = new Map(held.mounts);
  mounts.delete(name);
  put({
    ...held,
    mounts,
  });
}

const slots = new Map<string, HTMLElement>();
/** The node a mod's component renders into, which moves into the panel and out again, so opening and closing the panel does not remount it. */
export function slotOf(name: string): HTMLElement {
  const found = slots.get(name);
  if (found !== undefined) return found;
  const made = document.createElement("div");
  slots.set(name, made);
  return made;
}

// ---- What the console reads ---------------------------------------------------------------------------------

const instrumentOf = (row: ModSummary, mount: Mount | undefined): InstrumentMod => {
  const label = tabTitle(row.name);
  const fault = mount?.refused ?? mount?.thrown?.why;
  return {
    name: row.name,
    seat: `${MOD}${row.name}`,
    label,
    code: stencilOf(label),
    Icon: instrumentGlyph(row.glyph),
    ...(mount?.face === undefined
      ? {}
      : {
          face: {
            paint: mount.face.paint,
            moves: mount.face.moves,
            rev: mount.face.rev,
          },
        }),
    ...(mount?.loaded === true
      ? {
          slot: slotOf(row.name),
        }
      : {}),
    ...(row.made_by === undefined
      ? {}
      : {
          job: row.made_by,
        }),
    ...(fault === undefined
      ? {}
      : {
          fault,
        }),
    ...(mount?.off === true
      ? {
          off: true as const,
        }
      : {}),
  };
};

const NONE: readonly never[] = [];

/** The instrument mods on this machine, as Fleet lists them and as those seated have drawn. Safe mode reads no mod. */
export function useInstruments(): readonly InstrumentMod[] {
  const rows = useModList();
  const mounts = useHeld((now) => now.mounts);
  return useMemo(
    () =>
      skipsMods()
        ? NONE
        : rows
            .filter((one) => one.kind === "instrument")
            .map((one) => instrumentOf(one, mounts.get(one.name))),
    [
      rows,
      mounts,
    ],
  );
}

/** A seat held on a Job moves to the mod that Job made, the moment Fleet lists it. The same list where nothing moved. */
export function seatsFollowing<Seat extends string>(
  seats: readonly (Seat | null)[],
  rows: readonly Pick<ModSummary, "name" | "kind" | "made_by">[],
): readonly (Seat | `mod:${string}` | null)[] {
  const next = seats.map((seat) => {
    if (seat === null || !seat.startsWith(JOB)) return seat;
    const made = rows.find(
      (one) => one.kind === "instrument" && one.made_by === seat.slice(JOB.length),
    );
    return made === undefined ? seat : (`${MOD}${made.name}` as const);
  });
  return next.some((one, at) => one !== seats[at]) ? next : seats;
}

// ---- Make one ------------------------------------------------------------------------------------------------

const startedHere = new Set<string>();
/** A Job this window started: Fleet's list of Jobs may not have it yet, so its seat is not an old one for a Job long gone. */
export const startedHereJob = (id: string): boolean => startedHere.has(id);

/** Starts the Mod Job, and answers its id. A refusal is told as a toast and answers nothing, so no seat is taken. */
export async function startInstrument(said: string): Promise<string | undefined> {
  const started = await window.armada.startModSession({
    title: "Mod New instrument",
    said,
    aim: "instrument",
  });
  if (!started.ok) {
    askToTell(`Mod Job not started: ${refusal(started.outcome)}`);
    return undefined;
  }
  startedHere.add(started.value.session_id);
  return started.value.session_id;
}

/** A bay is showing this Job, until the answer is called. */
export function bayShows(job: string): () => void {
  put({
    ...held,
    shown: new Set([
      ...held.shown,
      job,
    ]),
  });
  return () =>
    put({
      ...held,
      shown: new Set(
        [
          ...held.shown,
        ].filter((one) => one !== job),
      ),
    });
}
/** The Jobs a bay is showing now. */
export const useBaysShowing = (): ReadonlySet<string> => useHeld((now) => now.shown);

/** A face while the Job has yet to write one: a bar going across. */
export const buildingFace: FacePaint = (draw, { w, h, t, ink }) => {
  const bar = w * 0.22;
  draw.fillStyle = ink.off;
  draw.fillRect(8, h / 2 - 2, w - 16, 4);
  draw.fillStyle = ink.run;
  draw.fillRect(8 + ((t * 0.125) % 1) * (w - 16 - bar), h / 2 - 2, bar, 4);
};
/** `buildingFace` as a bay draws it. */
export const BUILDING: Face = {
  paint: buildingFace,
  moves: true,
  rev: 0,
};
