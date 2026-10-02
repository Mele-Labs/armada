// A Studio's channels, and the capture window's bar — #1287, #1290, #1294. Out of `index.ts`
// because that file is at the length the gate refuses; `handleSketches` went the same way first.

import { app, type IpcMain } from "electron";
import { randomUUID } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

import { STUDIO_PROMOTIONS } from "@armada/protocol";
import type { StagedFrame, StudioCapture, StudioNodeByHand, StudioPosition, StudioPromotion } from "@armada/protocol";
import { CHANNELS } from "../shared/bridge";
import type { BridgeState } from "../shared/bridge";
import type { CaptureWindows } from "./capture/windows";
import type { FleetConnection } from "./connection";
import { handleSketches } from "./sketches";
import { stagedPicture } from "./staging";

type Hosts = {
  ipc: IpcMain;
  /** The open connection, or `null` before there is one. */
  connection: () => FleetConnection | null;
  /** What main last published — `index.ts`'s `published`. */
  published: () => BridgeState;
  captureWindows: CaptureWindows;
};

/** Enough of a capture to be worth sending. The rest is Fleet's to refuse. */
function isCapture(value: unknown): value is StudioCapture {
  if (typeof value !== "object" || value === null) return false;
  const capture = value as Record<string, unknown>;
  return typeof capture["selector"] === "string" && typeof capture["markup"] === "string";
}

/**
 * A PNG of the whole window the capture came from, written where an attachment
 * is staged. **The window, not the element**: a Note keeps what was on screen,
 * and the element's box within it says which part to look at.
 */
async function stagedFrame(event: Electron.IpcMainInvokeEvent): Promise<StagedFrame | null> {
  const image = await event.sender.capturePage();
  const png = image.toPNG();
  if (png.byteLength === 0) return null;
  const dir = join(app.getPath("temp"), "armada-frames", randomUUID());
  await mkdir(dir, { recursive: true });
  const staged = join(dir, "frame.png");
  await writeFile(staged, png);
  const size = image.getSize();
  return { staged_path: staged, width: size.width, height: size.height };
}

export function handleStudios({ ipc, connection, published, captureWindows }: Hosts): void {
  // A repository's Studios and the one open — #1287. An id that is not a string, or a position that
  // is not two whole numbers, is not put on a route: the call answers nothing, as a typo would.
  const text = (value: unknown): value is string => typeof value === "string" && value !== "";
  const unsent = { ok: false, why: "not_connected" } as const;
  // #1291: one channel across six operations, so the tag is what is checked —
  // it picks the route. The body is Fleet's to decode and refuse, as every
  // other act's body already is.
  const promoted = (value: unknown): value is StudioPromotion =>
    typeof value === "object" &&
    value !== null &&
    (STUDIO_PROMOTIONS as readonly string[]).includes((value as { act?: unknown }).act as string);
  /** A position in whole canvas units, or `null` where it is not one. */
  const whole = (value: unknown): StudioPosition | null => {
    const at = (value ?? {}) as { x?: unknown; y?: unknown };
    return Number.isInteger(at.x) && Number.isInteger(at.y)
      ? { x: at.x as number, y: at.y as number }
      : null;
  };
  /**
   * One of the three kinds a person writes by hand. Never a Picture or a Sketch,
   * whose staged files only main names.
   */
  const byHand = (value: unknown): value is StudioNodeByHand => {
    const node = (value ?? {}) as { kind?: unknown; said?: unknown; address?: unknown; path?: unknown };
    if (node.kind === "note") return text(node.said);
    if (node.kind === "link") return text(node.address);
    return node.kind === "file" && text(node.path);
  };
  ipc.handle(CHANNELS.watchStudios, (_event, manifestId: unknown) =>
    text(manifestId) || manifestId === null ? connection()?.studios.watchList(manifestId) : undefined,
  );
  ipc.handle(CHANNELS.watchStudio, (_event, studioId: unknown) =>
    text(studioId) || studioId === null ? connection()?.studios.watchStudio(studioId) : undefined,
  );
  ipc.handle(CHANNELS.createStudio, async (_event, manifestId: unknown) =>
    text(manifestId)
      ? ((await connection()?.studios.create(manifestId)) ?? { ok: false, outcome: unsent })
      : undefined,
  );
  ipc.handle(CHANNELS.renameStudio, async (_event, studioId: unknown, name: unknown) =>
    text(studioId) && text(name) ? ((await connection()?.studios.rename(studioId, name)) ?? unsent) : undefined,
  );
  // A node by hand — #1364. **The kind is checked here, not only typed**: the
  // preload is the boundary, and a renderer that sent `finding` would otherwise
  // reach a route Fleet refuses rather than one Bridge never offered.
  ipc.handle(CHANNELS.addStudioNode, async (_event, studioId: unknown, node: unknown, position: unknown) => {
    const at = whole(position);
    if (!text(studioId) || at === null || !byHand(node)) return undefined;
    return (await connection()?.studios.addNode(studioId, node, at)) ?? unsent;
  });
  // A pasted picture — 1 Oct 2026. Bytes in; main stages them (`staging.ts`).
  ipc.handle(CHANNELS.addStudioPicture, async (_event, studioId: unknown, bytes: unknown, position: unknown) => {
    const at = whole(position);
    if (!text(studioId) || at === null || !(bytes instanceof Uint8Array)) return undefined;
    const staged = await stagedPicture(bytes);
    if (staged === null) return undefined;
    return (await connection()?.studios.addNode(studioId, { kind: "picture", staged }, at)) ?? unsent;
  });
  handleSketches({ ipc, studios: () => connection()?.studios, whole, unsent });
  ipc.handle(
    CHANNELS.moveStudioNode,
    async (_event, studioId: unknown, nodeId: unknown, position: unknown, within: unknown) => {
      const at = whole(position);
      if (!text(studioId) || !text(nodeId) || at === null || !(within === null || text(within))) return undefined;
      return (await connection()?.studios.moveNode(studioId, nodeId, at, within)) ?? unsent;
    },
  );
  // Everything picked, deleted as one write — #1411. **Every name is checked
  // here before any of them crosses**, so a list carrying one thing that is not
  // a node reaches no route at all rather than half a delete.
  ipc.handle(CHANNELS.removeStudioNodes, async (_event, studioId: unknown, nodeIds: unknown) => {
    if (!text(studioId) || !Array.isArray(nodeIds) || nodeIds.length === 0 || !nodeIds.every(text)) return undefined;
    return (await connection()?.studios.removeNodes(studioId, nodeIds)) ?? unsent;
  });
  // Studio capture — #1290. **Main takes the frame, of the sender's own window
  // and no other**, so the one capability the preload gains is a Note on a
  // Studio rather than a screenshot the renderer could ask for and keep.
  ipc.handle(CHANNELS.captureStudioNote, async (event, studioId: unknown, said: unknown, capture: unknown) => {
    if (!text(studioId) || !text(said) || !isCapture(capture)) return undefined;
    const frame = await stagedFrame(event as Electron.IpcMainInvokeEvent).catch(() => null);
    return (await connection()?.studios.captureNote(studioId, said, capture, frame)) ?? unsent;
  });
  // The other half of the capture: the bytes of the picture one Note kept, read
  // by main and handed over for a `blob:`. **No new scheme and no CSP change** —
  // `img-src 'self' blob:` already draws one — and no path crosses either way.
  // A Sketch's picture is named by `picture`, its id on the drawing — 20.0.
  ipc.handle(CHANNELS.readStudioFrame, async (_event, studioId: unknown, nodeId: unknown, picture: unknown) =>
    text(studioId) && text(nodeId) && (picture === undefined || text(picture))
      ? ((await connection()?.studios.frameOf(studioId, nodeId, picture)) ?? { ok: false, outcome: unsent })
      : undefined,
  );
  // Starting one entry from a Studio — #1289, #1345. **The position is checked
  // here**, as `addStudioNode`'s is: a node lands where the person is looking,
  // and a body without one would put it at the origin.
  ipc.handle(CHANNELS.startStudioRun, async (_event, studioId: unknown, name: unknown, position: unknown) => {
    const at = whole(position);
    if (!text(studioId) || !text(name) || at === null) return undefined;
    return (await connection()?.studios.startRun(studioId, name, at)) ?? unsent;
  });
  ipc.handle(
    CHANNELS.startStudioServer,
    async (_event, studioId: unknown, name: unknown, position: unknown) => {
      const at = whole(position);
      if (!text(studioId) || !text(name) || at === null) return undefined;
      return (await connection()?.studios.startServer(studioId, name, at)) ?? unsent;
    },
  );
  // The capture window — #1294, `docs/practices/capture-window.md`. **Two ids
  // and no address main did not already hold**: the server, one of its own
  // links, and the Studio read off the one main is holding.
  ipc.handle(CHANNELS.openCaptureWindow, (_event, serverId: unknown, url: unknown) => {
    if (!text(serverId) || !text(url)) return undefined;
    const studio = connection()?.studios.servedBy(serverId) ?? null;
    return captureWindows.openOn(published(), serverId, url, studio);
  });
  // The rest are the window's own bar, answered for the window the call came
  // from. A bar names no window, and the page below it holds no preload.
  const barred = (event: Electron.IpcMainInvokeEvent | Electron.IpcMainEvent) => captureWindows.from(event.sender);
  ipc.handle(CHANNELS.captureWindowRead, (event) => barred(event)?.state() ?? null);
  ipc.handle(CHANNELS.captureWindowArm, (event, on: unknown) =>
    typeof on === "boolean" ? (barred(event)?.arm(on) ?? null) : null,
  );
  ipc.handle(CHANNELS.captureWindowAim, async (event, x: unknown, y: unknown) =>
    typeof x === "number" && typeof y === "number" ? ((await barred(event)?.aim(x, y)) ?? null) : null,
  );
  ipc.handle(CHANNELS.captureWindowHold, async (event, x: unknown, y: unknown) =>
    typeof x === "number" && typeof y === "number" ? ((await barred(event)?.hold(x, y)) ?? null) : null,
  );
  ipc.handle(CHANNELS.captureWindowRelease, (event) => barred(event)?.release());
  ipc.handle(CHANNELS.captureWindowSave, async (event, said: unknown) =>
    text(said) ? ((await barred(event)?.save(said)) ?? unsent) : undefined,
  );
  ipc.handle(CHANNELS.captureWindowReload, (event) => barred(event)?.reload());
  ipc.handle(CHANNELS.captureWindowFollowRefused, async (event) => {
    await barred(event)?.followRefused();
  });
  ipc.on(CHANNELS.captureWindowScroll, (event, wheel: unknown) => {
    const said = (wheel ?? {}) as Record<string, unknown>;
    const at = (key: string): number => (typeof said[key] === "number" ? (said[key] as number) : 0);
    barred(event)?.scroll({ x: at("x"), y: at("y"), deltaX: at("deltaX"), deltaY: at("deltaY") });
  });
  ipc.handle(CHANNELS.promoteOnStudio, async (_event, studioId: unknown, promotion: unknown) => {
    // The tag is checked here because it picks the route; the body Fleet
    // decodes and refuses on its own, as every other act's body is.
    if (!text(studioId) || !promoted(promotion)) return undefined;
    return (await connection()?.studios.promote(studioId, promotion)) ?? unsent;
  });
  ipc.handle(CHANNELS.decideStudioEdge, async (_event, studioId: unknown, edgeId: unknown, accepted: unknown) =>
    text(studioId) && text(edgeId) && typeof accepted === "boolean"
      ? ((await connection()?.studios.decideEdge(studioId, edgeId, accepted)) ?? unsent)
      : undefined,
  );
}
