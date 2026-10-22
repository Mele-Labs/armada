// The drones a Job's Field draws, read off the Session: its own and its subagents', and other Jobs' faded, each at a file spelled as the
// repository spells it, and where the Job's own drone stands and what it last wrote. Pure. `FieldPanel.tsx` draws them.

import type { Session, WorkDrone } from "@armada/screens/src/draft/sessions";

import { areaOf, relativeTo } from "../cockpit/deck/areas";
import type { Burst } from "../cockpit/deck/afterglow";
import type { DroneIn } from "../cockpit/deck/layout";
import { isWorking } from "./field";

/** The checkouts a Job's paths may be spelled under: the repository, each slot it held, and where it started. */
export function rootsOf(session: Session, root: string): string[] {
  const slots = new Set<number>();
  for (const one of session.attachments) if (one.kind === "slot" || one.kind === "branch") slots.add(one.slot);
  for (const bay of session.trail?.bays ?? []) slots.add(bay.slot);
  const bare = root.replace(/\/+$/, "");
  return [...[...slots].map((slot) => `${bare}/.armada/slots/slot-${slot}`), bare, ...(session.cwd === undefined ? [] : [session.cwd])].filter((one) => one !== "");
}

/** The word Fleet names a subagent by when it was not told its type, which says nothing on a card. */
const UNTYPED = "subagent";

/**
 * What a subagent's card calls it: the task it was given, as the strip names it, else its type, else that none was recorded. The
 * Job it works for is the card's label.
 */
export const subagentTitle = (one: Pick<WorkDrone, "agent" | "task">): string => one.task ?? (one.agent === undefined || one.agent.toLowerCase() === UNTYPED ? "No task recorded" : one.agent);

/** A drone of the Field: the Job's own live, the Job's own as the slider left it, or another Job's. Only the first shows what it is doing. */
export function flier(session: Session, one: WorkDrone, root: string, as: "live" | "then" | "ghost"): DroneIn {
  const own = `session:${session.id}`;
  const ghost = as === "ghost";
  const working = as === "live" && isWorking(session, one);
  const job = session.title ?? session.id;
  return {
    key: one.agent === undefined ? own : `${own}/${one.id}`,
    kind: "session",
    title: one.agent === undefined ? job : subagentTitle(one),
    ...(one.agent === undefined ? {} : { job }),
    root,
    said: "Working",
    hue: "calm",
    spin: working,
    hover: true,
    ping: false,
    stuck: false,
    flare: false,
    working,
    // Another Job's drone shows where it is and nothing of what it is doing.
    ...(one.path === "" || as !== "live" ? {} : { doing: one.doing, via: one.path }),
    ...(one.agent === undefined ? {} : { parent: own }),
    ...(one.file === undefined ? {} : { file: one.file }),
    touches: one.path === "" ? [] : [{ path: one.file ?? one.path, w: 4 }],
    shown: true,
    ...(ghost ? { ghost: true as const } : {}),
  };
}

/** A drone Fleet placed, with its file as the repository spells it and its plot read off that. */
export function rootedDrone(one: WorkDrone, roots: readonly string[]): WorkDrone {
  if (one.file === undefined) return one;
  const file = relativeTo(one.file, roots);
  return file === one.file ? one : { ...one, file, path: areaOf(file) ?? one.path };
}

/**
 * Where the Job's own drone is, and the file it last wrote: the newer of its last touch, which Fleet publishes as it is made, and its
 * last burst, entered once the burst is over. A file it only read is never the one it writes.
 */
export function mainFileOf(touch: WorkDrone | undefined, bursts: readonly Burst[]): { at: string | undefined; written: string | undefined } {
  const touchAt = touch?.file === undefined || touch.at === undefined ? -Infinity : Date.parse(touch.at);
  const last = bursts.at(-1);
  let lastWrite: Burst | undefined;
  for (let k = bursts.length - 1; k >= 0 && lastWrite === undefined; k -= 1) if (bursts[k]!.act === "write") lastWrite = bursts[k];
  const newer = (burst: Burst | undefined) => (burst !== undefined && (touch?.file === undefined || burst.at >= touchAt) ? burst.paths[0] : touch?.file);
  return { at: newer(last), written: newer(lastWrite) };
}
