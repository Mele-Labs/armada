// The Studios surface: its Studios, nodes, runs and the capture window.
// A slice imports protocol and screens, never another slice; `../api.ts` and `../bridge.ts` compose them.

import type {
  CaptureOpened,
  Followed,
  FrameRead,
  Outcome,
  SketchToKeep,
  StudioCapture,
  StudioNodeByHand,
  StudioPosition,
  StudioPromotion,
} from "@armada/protocol";
import type {
  CaptureAimed,
  CaptureHeld,
  CaptureWheel,
  CaptureWindowState,
} from "../capture-window";
import type { StudioAnswer, StudioRead, StudiosRead } from "@armada/screens/src/studio-reads";

export type StudiosApi = {
  /**
   * Hold one repository's Studios, by its Manifest id, or `null` to drop them. Published on
   * `BridgeState.studios` and kept current by `studio.changed`. #1287.
   */
  watchStudios: (manifestId: string | null) => Promise<void>;
  /** Hold one Studio whole, or `null` to drop it. Published on `BridgeState.studio`. */
  watchStudio: (studioId: string | null) => Promise<void>;
  /** Start an untitled Studio in one repository. Answers with it, which also lands on the list. */
  createStudio: (manifestId: string) => Promise<StudioAnswer>;
  /**
   * Name a Studio, or name it again — #1364. A person's act as much as Helm's,
   * and the only way an untitled one is named without asking Helm for it.
   */
  renameStudio: (studioId: string, name: string) => Promise<Outcome>;
  /**
   * Put a Note, a Link, a Sketch or a File on a Studio, where the person is looking — #1364.
   * `within` is the Zone a press inside one put it in, with `position` from
   * that Zone's corner as `moveStudioNode`'s is; `null` is the board.
   *
   * **Four kinds, and the type is what says so.** Every other kind is made by
   * the act that earns it, and Fleet refuses one from Bridge by name; a
   * capability wide enough to ask for a Finding would be a capability the
   * renderer has and the door has to keep taking away.
   */
  addStudioNode: (
    studioId: string,
    node: StudioNodeByHand,
    position: StudioPosition,
    within: string | null,
  ) => Promise<Outcome>;
  /**
   * Where a file pasted onto a Studio is on disk, or `""` for one that is not —
   * a screenshot. **The path and nothing else**: Electron hands the renderer a
   * name, and this reads no file.
   */
  pathOfFile: (file: File) => string;
  /**
   * Put a pasted picture on a Studio as a Picture, where the person is looking.
   * **Bytes in, never a path**: main stages them and names the staged file to
   * Fleet, so the renderer cannot point Fleet at a file of its choosing.
   */
  addStudioPicture: (studioId: string, bytes: Uint8Array, position: StudioPosition) => Promise<Outcome>;
  /**
   * Put a Sketch drawn on the pad on a Studio, and keep a Sketch's drawing as it
   * was left — 1 Oct 2026. **Bytes in for a new picture, never a path**, a
   * pasted Picture's rule: main stages them and names each staged file to Fleet.
   */
  addStudioSketch: (
    studioId: string,
    drawing: SketchToKeep,
    position: StudioPosition,
    within: string | null,
  ) => Promise<Outcome>;
  saveStudioSketch: (studioId: string, nodeId: string, drawing: SketchToKeep) => Promise<Outcome>;
  /**
   * Save where a person put a node down: the frame it landed in, `null` for the
   * board, and its spot from that frame's corner. Nothing else about a node is
   * written. `within` since protocol 23.0, #1620.
   */
  moveStudioNode: (studioId: string, nodeId: string, position: StudioPosition, within: string | null) => Promise<Outcome>;
  /**
   * Delete everything picked, and every edge on it, as one write — #1411. A
   * person's act, and only from the Studios surface.
   *
   * **The only delete, one node or eighteen.** **All of them or none**: Fleet
   * takes the whole selection in one transaction, so nothing here has to say
   * which half of a loop landed.
   */
  removeStudioNodes: (studioId: string, nodeIds: readonly string[]) => Promise<Outcome>;
  /** Accept a proposed relation, or reject it, which removes it. A person's act, as above. */
  decideStudioEdge: (studioId: string, edgeId: string, accepted: boolean) => Promise<Outcome>;
  /**
   * Put a Note on a Studio where a person pointed — #1290. **Main takes the
   * frame**, of the window the call came from and no other, so the renderer
   * never holds an image and asks for none.
   */
  captureStudioNote: (studioId: string, said: string, capture: StudioCapture) => Promise<Outcome>;
  /**
   * The picture one Note kept, as the bytes it is — #1352.
   *
   * **Answered to the caller rather than published**, for `readFrame`'s
   * reasons: a frame never changes, and one on `BridgeState` would keep every
   * Note's picture alive for as long as its Studio is open. The renderer makes
   * a `blob:` of what comes back and revokes it; the CSP already draws one,
   * so nothing about this reaches outside the app.
   */
  readStudioFrame: (studioId: string, nodeId: string, picture?: string) => Promise<FrameRead>;
  /**
   * Group, defer, write up, edit a draft, end a Contradiction or dispatch —
   * #1291. **One capability rather than six**: what `act` names is the route
   * main posts to, and every one answers with the Studio whole.
   */
  promoteOnStudio: (studioId: string, promotion: StudioPromotion) => Promise<Outcome>;
  /**
   * Run one Check or Command in this Studio's own checkout, as a Run node where
   * the person is looking — #1289.
   */
  startStudioRun: (studioId: string, name: string, position: StudioPosition) => Promise<Outcome>;
  /**
   * Start a Command with `serve` in the same checkout, as a Run node holding
   * the instance — #1345. **Its own capability**, as `startServer` is beside
   * `startCheckoutRun`: a server is held rather than run, and `start_run`
   * refuses a name carrying `serve`.
   */
  startStudioServer: (studioId: string, name: string, position: StudioPosition) => Promise<Outcome>;
  /**
   * Open what one Studio node points at, in whatever browses the web on this
   * machine — #1406. **A Studio id and a node id, never an address**, which is
   * `openPullRequest`'s rule: main reads the address off the Studio it
   * published and refuses anything that is not `https:`.
   */
  openStudioNode: (studioId: string, nodeId: string) => Promise<Followed>;
  /**
   * Open the capture window on a server Run, pinned to that Run's own loopback
   * origin — #1294, `docs/practices/capture-window.md`.
   *
   * **A server id and one of its links, never a composed address.** Main
   * resolves it off the live holder Fleet published, exactly as `openServerLink`
   * does, and loading a page in a window is strictly more than handing it to
   * the OS.
   *
   * **The one entry any Bridge surface has.** Everything the window then does
   * is `captureWindow` below, which only that window's own bar can reach.
   */
  openCaptureWindow: (serverId: string, url: string) => Promise<CaptureOpened>;
  /**
   * What the capture window's own bar may ask of the window it belongs to.
   *
   * **Every entry is answered for the calling bar and no other window.** Main
   * finds the window by the sender's own `webContents`, so a bar cannot name
   * one, and the page below it holds no preload and reaches none of this.
   */
  captureWindow: {
    /** Install the listener that draws the bar, and take its first state. */
    read: (onChanged: (state: CaptureWindowState) => void) => () => void;
    /** Turn capture on or off: the bar covers the page and a press points. */
    arm: (on: boolean) => Promise<CaptureWindowState | null>;
    /** What is under the pointer, in the page's own viewport coordinates. */
    aim: (x: number, y: number) => Promise<CaptureAimed | null>;
    /** Take the capture under the pointer. **The capture stays in main**; this is what to draw. */
    hold: (x: number, y: number) => Promise<CaptureHeld | null>;
    /** Drop what is held without capturing it. */
    release: () => Promise<void>;
    /** Put the held Note on the Studio this window opened on. Main takes the frame. */
    save: (said: string) => Promise<Outcome>;
    /** Reload the pinned origin. There is no address field and no history. */
    reload: () => Promise<void>;
    /** Hand the last refused address to the system browser, on a person's press. */
    followRefused: () => Promise<void>;
    /** A wheel taken while armed, so the page still scrolls under the outline. */
    scroll: (wheel: CaptureWheel) => void;
  };
};

export type StudiosState = {
  /**
   * One repository's Studios, held while the Studios surface shows, and kept current by
   * `studio.changed`. `main/studios.ts`. #1287.
   */
  studios: StudiosRead;
  /** The Studio open on that surface, replaced whole by every `studio.changed` about it. */
  studio: StudioRead;
};

export const STUDIOS_NOTHING_YET: StudiosState = {
  studios: { state: "none" },
  studio: { state: "none" },
};

export const STUDIOS_CHANNELS = {
  // A repository's Studios — #1287. Two reads a surface holds open, and one act per operation:
  // a Studio is started, named (#1364), a node added by hand (#1364), moved or removed, a
  // proposed relation decided.
  watchStudios: "bridge:watch-studios",
  watchStudio: "bridge:watch-studio",
  createStudio: "bridge:create-studio",
  renameStudio: "bridge:rename-studio",
  addStudioNode: "bridge:add-studio-node",
  addStudioPicture: "bridge:add-studio-picture",
  addStudioSketch: "bridge:add-studio-sketch",
  saveStudioSketch: "bridge:save-studio-sketch",
  moveStudioNode: "bridge:move-studio-node",
  // Everything picked, deleted as one write — #1411. Its own channel because it
  // is its own operation on the wire, not a loop over the one above.
  removeStudioNodes: "bridge:remove-studio-nodes",
  decideStudioEdge: "bridge:decide-studio-edge",
  // Studio capture — #1290. One channel: the renderer says where it pointed and
  // main takes the frame of its own window, so no image ever reaches the renderer.
  captureStudioNote: "bridge:capture-studio-note",
  readStudioFrame: "bridge:read-studio-frame",
  // One rung of promotion — #1291. One channel across six operations: what a
  // person does on a Studio is one capability, and `act` picks the route.
  promoteOnStudio: "bridge:promote-on-studio",
  // Starting one entry from a Studio — #1289, #1345. **Two channels and not
  // one**, because they are two operations: a run ends and a server is held,
  // and a single channel taking "which" as an argument would make that a flag.
  startStudioRun: "bridge:start-studio-run",
  startStudioServer: "bridge:start-studio-server",
  openStudioNode: "bridge:open-studio-node",
  // The capture window — #1294. **Opened from Bridge, driven from its own
  // bar**: the first channel is the Studio surface's, and the rest are the
  // bar's own document talking to the window it belongs to. The page below the
  // bar reaches none of them, because it holds no preload at all.
  openCaptureWindow: "bridge:open-capture-window",
  captureWindowChanged: "bridge:capture-window-changed",
  captureWindowRead: "bridge:capture-window-read",
  captureWindowArm: "bridge:capture-window-arm",
  captureWindowAim: "bridge:capture-window-aim",
  captureWindowHold: "bridge:capture-window-hold",
  captureWindowRelease: "bridge:capture-window-release",
  captureWindowSave: "bridge:capture-window-save",
  captureWindowReload: "bridge:capture-window-reload",
  captureWindowFollowRefused: "bridge:capture-window-follow-refused",
  captureWindowScroll: "bridge:capture-window-scroll",
} as const;
