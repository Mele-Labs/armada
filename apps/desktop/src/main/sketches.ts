// A Studio's Sketch, as main keeps it — 1 Oct 2026: a Studio's Sketch is the dispatch composer's
// pad. Its own module because `index.ts` is at the length the gate refuses.
//
// **Bytes in for each new picture, never a path**, a pasted Picture's rule: main stages them, and
// a picture Fleet already keeps is named by its id alone. Nothing the renderer sends names a file.

import type { IpcMain } from "electron";
import type { Outcome, SketchDrawn, StudioPosition } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import { stagedSketch } from "./staging";

/** The two writes a Sketch takes, as the open connection makes them. */
type Writes = {
  addNode: (studioId: string, node: { kind: "sketch"; drawing: SketchDrawn }, at: StudioPosition) => Promise<Outcome>;
  editSketch: (studioId: string, nodeId: string, drawing: SketchDrawn) => Promise<Outcome>;
};

type Hosts = {
  ipc: IpcMain;
  /** The Studio writes, or `undefined` with no connection open. */
  studios: () => Writes | undefined;
  /** A position in whole canvas units, or `null` where it is not one — `index.ts`'s own check. */
  whole: (value: unknown) => StudioPosition | null;
  /** What a write answers with no connection open. */
  unsent: Outcome;
};

const text = (value: unknown): value is string => typeof value === "string" && value !== "";

export function handleSketches({ ipc, studios, whole, unsent }: Hosts): void {
  ipc.handle(CHANNELS.addStudioSketch, async (_event, studioId: unknown, drawing: unknown, position: unknown) => {
    const at = whole(position);
    if (!text(studioId) || at === null) return undefined;
    const drawn = await stagedSketch(drawing);
    if (drawn === null) return undefined;
    return (await studios()?.addNode(studioId, { kind: "sketch", drawing: drawn }, at)) ?? unsent;
  });
  ipc.handle(CHANNELS.saveStudioSketch, async (_event, studioId: unknown, nodeId: unknown, drawing: unknown) => {
    if (!text(studioId) || !text(nodeId)) return undefined;
    const drawn = await stagedSketch(drawing);
    if (drawn === null) return undefined;
    return (await studios()?.editSketch(studioId, nodeId, drawn)) ?? unsent;
  });
}
