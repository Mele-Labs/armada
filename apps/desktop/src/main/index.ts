import { app, BrowserWindow, ipcMain, nativeImage, Notification } from "electron";
import { randomUUID } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";

import { CHANNELS, NOTHING_YET } from "../shared/bridge";
import type { BridgeState, PickedView, Summons } from "../shared/bridge";
import type { HelmCarry } from "../shared/api/helm";
import type { BuildSource, ChangeSlotPool, Outcome, RescueSlot, RetroSubject } from "@armada/protocol";
import type { AskTurn } from "@armada/screens/src/lessons/review-wire";
import type { HelmContext, LandCheckAt, ThoughtTarget } from "@armada/protocol";
import { landCheckAt } from "./land-following";
import type { HelmCallAnswer, SavePreference, SaveSettings } from "@armada/protocol";
import { ANNOTATE_FLAG } from "../shared/annotations";
import { handleAnnotations } from "./annotations";
import { approvedText, reviewNoteOf } from "./capture/session-note";
import { CaptureWindows } from "./capture/windows";
import { coalesce } from "./coalesce";
import { StateSends } from "./state-sends";
import { stagePng } from "./staging";
import { FleetConnection } from "./connection";
import { handleTaps } from "./haptics";
import { installSounds } from "./dev-sounds";
import { openSettingsFile } from "./open";
import { openStudioNode } from "./forge";
import { openLink } from "./links";
import { askGateway } from "./phone-gateway";
import type { PhoneRequest } from "@armada/settings/api";
import { restartFleet } from "./restart-fleet";
import { openServerLink } from "./servers";
import { handleRehearsal } from "./rehearsal-channels";
import { handleRepositories } from "./repository-channels";
import { handleSessions } from "./session-channels";
import { handleSleep } from "./sleep-channels";
import { handleSketchShare } from "./sketch-share";
import { handleAccounts } from "./accounts-channels";
import { handlePlanUsage } from "./plan-usage-channels";
import { handleModelDownshift } from "./model-downshift-channels";
import { DownshiftNotice } from "./downshift-notice";
import { SessionAttention } from "./session-attention";
import { soundOf } from "./telling";
import { settingOf } from "./settings";
import { handleTimeline } from "./timeline-channels";
import { handleClashes } from "./clashes-channels";
import { handleReviews } from "./reviews-channels";
import { handleTerritory } from "./territory-channels";
import { handleBayHistory } from "./bay-history-channels";
import { handleUsageSummary } from "./usage-summary-channels";
import { handleFlightControls } from "./flight-controls-channels";
import { handleOnboarding } from "./onboarding-channels";
import { handleIssues } from "./issue-channels";
import { SessionPages } from "./session-page";
import { handleStudios } from "./studio-channels";
import { HeldNotes } from "./capture/held-notes";
import { handleHeldNotes, tellHeld } from "./held-notes-channels";
import { remember as rememberWindow, restore as restoreWindow } from "./window-state";

// Bridge's window, and the one connection under it.
//
// Bridge and Fleet have independent lifetimes: Sessions keep progressing with the
// window closed, and opening it reconnects to whatever Fleet is already running
// rather than starting one. Nothing here spawns Fleet.

/**
 * The mark, as the macOS app tile.
 *
 * Emitted beside this bundle by `electron.vite.config.ts` — see the note there
 * for why it is a file on disk rather than an import. **The accent-filled
 * variant is sanctioned for the app tile and nowhere else**, which is what this
 * carries and why nothing in the renderer may reach for it.
 *
 * **A PNG, and the `.icns` beside it is not what this reads.**
 * `nativeImage.createFromPath` returns an empty image for `.icns` — measured
 * against Electron 40.10.6, for the checked-in file and for one rebuilt with
 * `iconutil`, while the same iconset's PNG loads at 512×512. This line said
 * `AppIcon.icns` for a fortnight and the tile stayed Electron's the whole time.
 */
const APP_ICON = join(__dirname, "AppIcon.png");

/**
 * Wear the mark. **Runtime, not packaging** — `BrowserWindow`'s `icon` option
 * is ignored on macOS, and a packaged `Armada.app` already carries the `.icns`
 * through its own `Info.plist`. This is what makes the dock tile right when
 * Bridge is run unpackaged, which is how it is run all day: `electron-vite
 * preview` launches Electron's own bundle, and that bundle's icon is
 * Electron's.
 *
 * A failed read leaves Electron's own icon rather than stopping the app: a
 * window that will not open because of a picture is the wrong trade. **It says
 * so now.** Silence is what let a tile nobody had looked at go two builds
 * wearing Electron's mark, with every gate green — this is the only line main
 * writes, and it earns it by being the failure that has already happened.
 */
function wearTheMark(): void {
  if (process.platform !== "darwin" || app.dock === undefined) return;
  const icon = nativeImage.createFromPath(APP_ICON);
  if (icon.isEmpty()) {
    console.warn(`the app tile did not load, so the dock keeps Electron's: ${APP_ICON}`);
    return;
  }
  app.dock.setIcon(icon);
}

/**
 * Write pasted or picked bytes to a fresh staging directory, before any Session
 * exists to key storage on. One directory per file rather than one per
 * batch, keyed by a fresh uuid, so two attachments sharing a filename in one
 * paste never collide on disk.
 */
async function stageAttachment(
  bytes: ArrayBuffer,
  filename: string,
  _mimeType: string,
): Promise<{ path: string }> {
  const dir = join(app.getPath("temp"), "armada-attachments", randomUUID());
  await mkdir(dir, { recursive: true });
  const path = join(dir, filename);
  await writeFile(path, Buffer.from(bytes));
  return { path };
}

let connection: FleetConnection | null = null;

/** Nothing reached Fleet, because there is no connection to reach it on. */
const UNSENT: Outcome = { ok: false, why: "not_connected" };

/**
 * The capture windows open on server Runs — #1294,
 * `docs/practices/capture-window.md`. Each holds a page with no preload and
 * Bridge's own bar above it; the Studio a Note lands on is read off the Studio
 * main is holding, never off a name a renderer sent.
 */
/** The walk windows with focus right now: Bridge's own windows dim while any does. */
const walkFocused = new Set<string>();
const tellWindows = (channel: string, ...values: unknown[]): void => {
  for (const window of BrowserWindow.getAllWindows()) if (!window.isDestroyed()) window.webContents.send(channel, ...values);
};
/** A Job's walk notes, held until the owner sends them as one turn. `userData`, as the window's place is kept. */
const heldNotes = new HeldNotes(join(app.getPath("userData"), "held-notes.json"), (sessionId) => tellHeld(heldNotes, captureWindows, sessionId));
const captureWindows = new CaptureWindows({
  capture: async (studioId, said, capture, frame) =>
    (await connection?.studios.captureNote(studioId, said, capture, frame)) ?? UNSENT,
  holdNote: (note) => void heldNotes.hold(note),
  heldOn: (sessionId) => heldNotes.of(sessionId).length,
  // A pull request's walk no Session holds: the note stays in Armada. No Approve.
  reviewNote: async (review, said, capture) => {
    tellWindows(CHANNELS.reviewWalkNote, reviewNoteOf(review, said, capture, new Date().toISOString()));
    return { ok: true };
  },
  reviewClosed: (review) => {
    void connection?.reviews.closeWalk(review.manifestId, review.walkId);
    tellWindows(CHANNELS.reviewWalkClosed, review.walkId);
  },
  sessionApproved: async (sessionId, address, take) => {
    const sent = await connection?.sessions.send({ session_id: sessionId, text: approvedText(address, take) });
    return sent === undefined ? UNSENT : sent.ok ? { ok: true } : sent.outcome;
  },
  // The Session's question in its window settles as the item it is under Waiting on you.
  answerAsk: async (sessionId, itemId, answer) => {
    const sent = await connection?.sessions.answerWaiting({ session_id: sessionId, item_id: itemId, ...answer });
    return sent === undefined ? UNSENT : sent.ok ? { ok: true } : sent.outcome;
  },
  sessionClosed: (sessionId, address) => {
    void connection?.sessions.windowClosed({ session_id: sessionId, url: address });
  },
  sessionFailed: (sessionId, address, why) => {
    void connection?.sessions.windowFailed({ session_id: sessionId, url: address, why });
  },
  // The Cockpit holds back a walk's call while its window is open, so it hears when one opens or closes.
  windowsChanged: () => {
    const open = captureWindows.sessionAddresses();
    for (const window of BrowserWindow.getAllWindows()) {
      if (!window.isDestroyed()) window.webContents.send(CHANNELS.sessionWindowsChanged, open);
    }
  },
  stage: stagePng,
  focused: (serverId, on) => {
    if (on) walkFocused.add(serverId);
    else walkFocused.delete(serverId);
    for (const window of BrowserWindow.getAllWindows()) {
      if (!window.isDestroyed()) window.webContents.send(CHANNELS.walkFocused, walkFocused.size > 0);
    }
  },
});

function createWindow(): BrowserWindow {
  const placed = restoreWindow();
  const window = new BrowserWindow({
    ...placed.bounds,
    show: false,
    // Frameless: the title row Bridge draws itself — #1087 — replaces macOS's
    // grey bar, which said only "Armada" while the app's own controls sat
    // lower. `x, y` is the top-left corner of the button cluster, not its
    // centre — a 12px-diameter button 22px down a 44px row sits low, which is
    // the correction pass's #1106 finding. 16 centres it: (44 - 12) / 2. The
    // row's own height and `TitleBar.css`'s left gutter are read against this
    // figure together, same gap that css already reports.
    titleBarStyle: "hiddenInset",
    trafficLightPosition: { x: 18, y: 16 },
    webPreferences: {
      preload: join(__dirname, "../preload/index.js"),
      sandbox: true,
      contextIsolation: true,
      nodeIntegration: false,
      // The dev-only annotation layer, #1226: the preload exposes it on this flag alone.
      additionalArguments: app.isPackaged ? [] : [ANNOTATE_FLAG],
    },
  });

  if (placed.maximized) window.maximize();
  if (placed.fullScreen) window.setFullScreen(true);
  rememberWindow(window);
  window.on("ready-to-show", () => window.show());

  // History from the OS: macOS's swipe gesture, and the browser keys on Windows and Linux.
  const step = (to: "back" | "forward"): void => {
    if (!window.isDestroyed()) window.webContents.send(CHANNELS.history, to);
  };
  window.on("swipe", (_event, direction) => {
    if (direction === "left") step("back");
    else if (direction === "right") step("forward");
  });
  window.on("app-command", (_event, command) => {
    if (command === "browser-backward") step("back");
    else if (command === "browser-forward") step("forward");
  });

  // This window's own pick and its per-window commands go with it.
  window.on("closed", () => {
    pickedViews.delete(window.id);
    connection?.dropWindow(window.id);
  });

  // **This window goes nowhere.** It loads one file and stays on it for the
  // life of the process, so every navigation and every new window is refused
  // outright rather than filtered — there is no address this app is meant to
  // reach, which makes an allowlist a thing that could be wrong and a refusal a
  // thing that cannot.
  //
  // Here because a page may draw a real `<a href>`. That anchor cancels its own default and hands the click to main,
  // but the middle click, the modifier click and the `target` a future one
  // forgets to cancel all route around a click handler — and `default-src
  // 'self'` does not cover top-level navigation. A window that navigated to a
  // forge would be a window with no rail, no shell and no way back: Electron's
  // version of the frozen surface this app was built to escape.
  //
  // `openExternal` is reached on a channel, never by navigating: `forge.ts` from
  // main's own state, `links.ts` from a link in a model's text, `http(s):` only.
  window.webContents.setWindowOpenHandler(() => ({ action: "deny" }));
  window.webContents.on("will-navigate", (event) => event.preventDefault());

  // **Always the built renderer, and `dev` builds it first.** electron-vite
  // serves a dev server and publishes `ELECTRON_RENDERER_URL`; loading it would
  // be the obvious branch and it does not work here, because the React plugin
  // injects an inline module preamble in dev and `default-src 'self'` refuses
  // it. Relaxing the CSP is a security review rather than a local convenience,
  // so the build is what moves. Reported.
  //
  // **`--no-mods` is safe mode**: the renderer reads `?nomods` and loads without any mod's theme.
  // A query on the page, so the preload surface gains nothing.
  void window.loadFile(join(__dirname, "../renderer/index.html"), process.argv.includes("--no-mods") ? { query: { nomods: "1" } } : {});
  return window;
}

/**
 * What main last published.
 *
 * Kept here rather than read back off the connection, because the one caller
 * needs a Manifest and nothing else — and `FleetConnection.state`
 * is five round trips to Fleet, which is the wrong price for a click on a path.
 */
let published: BridgeState = NOTHING_YET;

/** Every window's own `PickedView`, kept apart from `published` and from every other window's. */
const pickedViews = new Map<number, PickedView>();

const NO_PICK: PickedView = {
  repository: null,
  manifestReading: null,
  health: { state: "none" },
  checkoutRunSheet: { state: "none" },
  checkoutRunFollowed: { state: "none" },
  manifestDrift: { state: "none" },
};

function viewFor(windowId: number): PickedView {
  return pickedViews.get(windowId) ?? NO_PICK;
}

/** `state` with one window's own view folded in — `leftOut` nests under `holds`, everything else is top-level. */
function withView(state: BridgeState, view: PickedView): BridgeState {
  const { leftOut, ...rest } = view;
  return { ...state, ...rest, holds: { ...state.holds, leftOut } };
}

/** What each window holds of the state, so a send is only what moved since: `shared/state-change.ts`. */
const sends = new StateSends<BridgeState>();

function sendState(contents: Electron.WebContents, state: BridgeState): void {
  const change = sends.next(contents, state);
  if (change !== null) contents.send(CHANNELS.changed, change);
}

/** Every window sees the same state, save its own pick — `pickedViews`, overlaid here. */
const toWindows = coalesce((state: BridgeState) => {
  for (const window of BrowserWindow.getAllWindows()) {
    if (!window.isDestroyed()) sendState(window.webContents, withView(state, viewFor(window.id)));
  }
  // A walk window's panel reads what Bridge's windows read, and picks no repository of its own.
  for (const panel of captureWindows.panels()) sendState(panel, withView(state, NO_PICK));
});

/**
 * Main's own readers move on every change; the windows are told once a frame,
 * because a backfill publishes once per row and each send clones the whole state.
 */
function publish(state: BridgeState): void {
  published = state;
  // A Run that has stopped serving ends capture in its own window — #1294.
  captureWindows.changed(state);
  toWindows(state);
}

/** One window's own pick moved. Told to that window alone — nothing else here reaches the rest. */
function publishToWindow(windowId: number, change: Partial<PickedView>): void {
  const view = { ...viewFor(windowId), ...change };
  pickedViews.set(windowId, view);
  const window = BrowserWindow.getAllWindows().find((one) => one.id === windowId);
  if (window !== undefined && !window.isDestroyed()) {
    sendState(window.webContents, withView(published, view));
  }
}

/** Every window open right now — `connection.ts`'s `Wiring.windowIds`. */
function windowIds(): readonly number[] {
  return BrowserWindow.getAllWindows().map((window) => window.id);
}

/** The window an IPC call arrived from, or `0` where Electron cannot say — never a real window's id. */
function windowIdOf(event: Electron.IpcMainInvokeEvent): number {
  return BrowserWindow.fromWebContents(event.sender)?.id ?? 0;
}

/**
 * A press on a notification, waiting for a window to hand it to.
 *
 * **The press outlives the absence of a window, and that is the point.** The
 * whole feature is being told with Bridge closed, so a click that arrived with
 * nothing on screen has to open one and land in the right place rather than
 * open one at whatever it was last showing.
 */
let summoned: Summons | null = null;

/**
 * Go where a pressed notification says.
 *
 * A window that exists is raised and told. With none, one is opened and the
 * press is held until that window asks for state — which every window does on
 * mount, having already installed its listener, so nothing races.
 */
function summon(to: Summons): void {
  summoned = to;
  const [window] = BrowserWindow.getAllWindows();
  if (window === undefined || window.isDestroyed()) {
    createWindow();
    return;
  }
  if (window.isMinimized()) window.restore();
  window.show();
  window.focus();
  hand(window);
}

/** Bring Bridge forward, opening a window if none is up. A press that names nothing to land on. */
function focusBridge(): void {
  const [window] = BrowserWindow.getAllWindows();
  if (window === undefined || window.isDestroyed()) {
    createWindow();
    return;
  }
  if (window.isMinimized()) window.restore();
  window.show();
  window.focus();
}

/** Hand over a held press, if there is one. Nothing is handed twice. */
function hand(window: BrowserWindow): void {
  if (summoned === null || window.isDestroyed()) return;
  window.webContents.send(CHANNELS.summoned, summoned);
  summoned = null;
}

/**
 * What the OS says when the weekly limit makes the default model's step down due. The rule, the words and
 * the tone are `downshift-notice.ts`'s; this is the effect. Nothing here asks for permission: the first `show()` is what makes macOS ask, once.
 */
const downshiftNotice = new DownshiftNotice({
  show: (told) => {
    if (!Notification.isSupported()) return;
    const banner = new Notification({ title: told.title, body: told.body, sound: told.sound });
    banner.on("click", focusBridge);
    banner.show();
  },
});

/**
 * What the OS says when a Job starts waiting on the owner. The rule, the words and the tone are `session-attention.ts`'s; this is
 * the effect. **A banner only while no Bridge window has focus, and only while `features.notifications` is not off**; the dock count
 * needs neither. A press lands on the Job.
 */
const sessionAttention = new SessionAttention({
  show: (told) => {
    if (BrowserWindow.getFocusedWindow() !== null) return;
    if (settingOf(published.settings, "features.notifications") === false) return;
    if (!Notification.isSupported()) return;
    const banner = new Notification({ title: told.title, body: told.body, sound: soundOf(told.tone) });
    banner.on("click", () => summon({ sessionId: told.sessionId }));
    banner.show();
  },
  count: (waiting) => app.dock?.setBadge(waiting === 0 ? "" : String(waiting)),
});

void app.whenReady().then(() => {
  wearTheMark();
  if (!app.isPackaged) {
    // A sent note's screenshot: the part of the window the note points at, #1250.
    handleAnnotations(ipcMain, app.getAppPath(), async (event, box) => {
      const image = await (event as Electron.IpcMainInvokeEvent).sender.capturePage({
        x: Math.max(0, Math.round(box.x)),
        y: Math.max(0, Math.round(box.y)),
        width: Math.max(1, Math.round(box.width)),
        height: Math.max(1, Math.round(box.height)),
      });
      const png = image.toPNG();
      return png.buffer.slice(png.byteOffset, png.byteOffset + png.byteLength) as ArrayBuffer;
    });
  }

  connection = new FleetConnection({
    home: process.env["HOME"],
    publish,
    publishToWindow,
    windowIds,
    now: () => Date.now(),
    threadsDir: join(app.getPath("userData"), "session-threads"),
  });
  // A Session showed a page: it joins the one walk window, which opens behind by itself where none is up.
  connection.sessions.onWindow(
    (sessionId, title, url, takes, origins, signIn, steps, pressed) =>
      void captureWindows.openForSession(sessionId, title, url, takes, origins, signIn, steps, pressed),
  );
  // A Session asked about a place on its walk: the walk on screen goes there, and one that is not is marked.
  connection.sessions.onAsked(
    (sessionId, title, window, place, itemId) => void captureWindows.askedFor(sessionId, title, window, place, itemId),
  );
  // Sleep mode or a repository's Autopilot changed in Fleet: every window gets the night, or it, whole.
  const everyWindow = (channel: string, ...values: unknown[]): void => {
    for (const window of BrowserWindow.getAllWindows()) if (!window.isDestroyed()) window.webContents.send(channel, ...values);
  };
  // Every Session list after it changes: a Job that started waiting is told when no window is looking.
  connection.sessions.onRead((sessions, readAt) => sessionAttention.read(sessions, readAt));
  // What a shell block run from a Session prints, to every window: only the one that started it holds the id.
  connection.sessions.onRun((runId, message) => everyWindow(CHANNELS.sessionRunMessage, runId, message));
  connection.sleep.onChanged((state) => everyWindow(CHANNELS.sleepChanged, state));
  connection.planUsage.onChanged((usage) => everyWindow(CHANNELS.planUsageChanged, usage));
  connection.modelDownshift.onChanged((state) => {
    downshiftNotice.read(state);
    everyWindow(CHANNELS.modelDownshiftChanged, state);
  });
  connection.modelDownshift.onSeeded((state) => downshiftNotice.seed(state));
  connection.timeline.onAppended((event) => everyWindow(CHANNELS.timelineAppended, event));
  connection.reviews.onRun((run) => everyWindow(CHANNELS.reviewRunChanged, run));
  connection.reviews.onWalk((walk) => everyWindow(CHANNELS.reviewWalkChanged, walk));
  connection.issues.onAutopilotChanged((autopilot) => everyWindow(CHANNELS.autopilotChanged, autopilot));
  handleTaps({ ipc: ipcMain, app });
  if (!app.isPackaged) installSounds(join(app.getAppPath(), "sounds"), join(app.getPath("home"), "Library", "Sounds"));

  // The renderer initiates exactly these things and no others. There is no
  // arbitrary-channel invoke, which is what keeps the surface readable.
  // Awaited, because `state()` brings the connection current first — a window
  // reload is a fresh reader and gets what exists rather than what main last
  // heard about. See `FleetConnection.state`.
  // The whole state as a window's own base for the changes that follow, on mount and wherever one reaches it out of order.
  ipcMain.handle(CHANNELS.syncState, async (event) => {
    const state = (await connection?.state()) ?? published;
    return sends.full(event.sender, withView(state, viewFor(windowIdOf(event))));
  });
  ipcMain.handle(CHANNELS.state, async (event) => {
    const state = await connection?.state();
    // A fresh window asks for state on mount with its listener already
    // installed, so this is the moment a press held while there was no window
    // can be handed over — and the only one that needs no second channel to
    // announce readiness.
    const window = BrowserWindow.fromWebContents(event.sender);
    if (window !== null) hand(window);
    // This window's own pick, overlaid — a reload is a fresh reader, not a reset of it.
    return state === undefined ? state : withView(state, viewFor(windowIdOf(event)));
  });
  // Staging happens before a Session exists — there is no id yet to key storage
  // on, and one is minted at `propose` time. Fleet is not involved here at
  // all; this only writes bytes to a temp file the request later names.
  ipcMain.handle(
    CHANNELS.stageAttachment,
    (_event, bytes: ArrayBuffer, filename: string, mimeType: string) =>
      stageAttachment(bytes, filename, mimeType),
  );
  ipcMain.handle(CHANNELS.searchFiles, (event, query: string, manifestId: unknown) =>
    connection?.commands.searchFiles(query, connection.repositories.pickedByWindow.of(windowIdOf(event)), typeof manifestId === "string" ? manifestId : undefined),
  );
  ipcMain.handle(CHANNELS.readRequestRepository, (_event, words: unknown) =>
    connection?.commands.readRequestRepository(typeof words === "string" ? words : ""),
  );
  // The pool reshaped from Cleanup's bay grid, and read again after for the
  // reclaim's reason: whether a slot went is Fleet's reading.
  ipcMain.handle(CHANNELS.changeSlotPool, async (_event, manifestId: string, change: ChangeSlotPool) => {
    const outcome = await connection?.commands.changeSlotPool(manifestId, change);
    await connection?.rereadHeld();
    return outcome;
  });
  // The rescue of a stranded slot, read again after for the same reason: whether
  // the Scout is reading, or the slot went, is Fleet's reading.
  ipcMain.handle(CHANNELS.rescueSlot, async (_event, manifestId: string, rescue: RescueSlot) => {
    const outcome = await connection?.commands.rescueSlot(manifestId, rescue);
    await connection?.rereadHeld();
    return outcome;
  });
  // One call helm was held on, answered in the dock. The session is
  // waiting inside its own tool call, and nothing on the board moves. #1389.
  ipcMain.handle(
    CHANNELS.answerHelmCall,
    (_event, call: string, answer: HelmCallAnswer, note?: string) =>
      connection?.commands.answerHelmCall(call, answer, note),
  );
  // The owner's answer to one retro item.
  ipcMain.handle(CHANNELS.agreeLesson, (_event, lessonId: string) => connection?.commands.agreeLesson(lessonId));
  ipcMain.handle(CHANNELS.disagreeLesson, (_event, lessonId: string) =>
    connection?.commands.disagreeLesson(lessonId),
  );
  // Agreeing by a Session: Fleet starts it and drafts the first message, and sends nothing.
  ipcMain.handle(CHANNELS.startLessonSession, (_event, lessonId: string) =>
    connection?.commands.startLessonSession(lessonId),
  );
  // A person's Bridge preferences. Fleet-wide, so no id rides this
  // channel — the only one among the acts above that names none.
  ipcMain.handle(CHANNELS.savePreference, (_event, save: SavePreference) =>
    connection?.commands.savePreference(save),
  );
  // settings.json. Fleet-wide, so no id rides this either — `settings.ts`.
  ipcMain.handle(CHANNELS.saveSettings, async (_event, changes: SaveSettings["changes"]): Promise<Outcome> =>
    (await connection?.commands.saveSettings(changes)) ?? { ok: false, why: "not_connected" },
  );
  // The file itself, in the configured editor. No path crosses: main opens the one Fleet named.
  ipcMain.handle(CHANNELS.openSettingsFile, () => openSettingsFile(published));
  // The Phone Gateway, one named operation at a time — `phone-gateway.ts` makes the call.
  ipcMain.handle(CHANNELS.phone, (_event, request: PhoneRequest) => askGateway(request));
  // The mods on this machine. Fleet-wide, so no id rides these either — `mods.ts`.
  ipcMain.handle(CHANNELS.validateMod, async (_event, name: string) => (await connection?.mods.validate(name)) ?? null);
  ipcMain.handle(CHANNELS.setModEnabled, async (_event, name: string, enabled: boolean): Promise<Outcome> =>
    (await connection?.mods.setEnabled(name, enabled)) ?? { ok: false, why: "not_connected" },
  );
  ipcMain.handle(CHANNELS.promoteMod, async (_event, name: string): Promise<Outcome> =>
    (await connection?.mods.promote(name)) ?? { ok: false, why: "not_connected" },
  );
  // Which running Check's log is open. A fourth socket, carrying lines only,
  // and read-only.
  ipcMain.handle(
    CHANNELS.followCheckOutput,
    (_event, jobId: string | null, kept: string | null) =>
      connection?.followCheckOutput(jobId, kept),
  );
  // Which merge line Check's log is open, by the line's three names. Its own socket, read-only.
  // **Only the three strings cross**: anything else the renderer sends is not asked about.
  ipcMain.handle(CHANNELS.followLandCheck, (_event, at: LandCheckAt | null) =>
    connection?.followLandCheck(landCheckAt(at)),
  );
  handleRehearsal({ ipc: ipcMain, connection: () => connection, windowIdOf });
  // Overview's health and per-repository drift, held open by that surface.
  ipcMain.handle(CHANNELS.watchOverview, (event, want: unknown) =>
    connection?.overviewFor(windowIdOf(event)).watch(want === true),
  );
  // Helm's conversation: say something, end its Session, and point it without moving the rail's own
  // pick. Fleet-wide, unlike the window's own reads above: one Helm Session per repository,
  // whichever window's dock is open on it.
  ipcMain.handle(CHANNELS.askHelm, (_event, text: string, context?: HelmContext, carry?: HelmCarry) =>
    connection?.askHelm(text, context, carry),
  );
  ipcMain.handle(CHANNELS.helmDebugInfo, () => connection?.helmDebugInfo());
  ipcMain.handle(CHANNELS.readThought, (_event, text: string) => connection?.readThought(text));
  ipcMain.handle(CHANNELS.recordThoughtSent, (_event, manifestId: string, text: string, target: ThoughtTarget, carry?: HelmCarry) =>
    connection?.recordThoughtSent(manifestId, text, target, carry),
  );
  ipcMain.handle(CHANNELS.startHelmFresh, () => connection?.startHelmFresh());
  ipcMain.handle(CHANNELS.pointHelm, (_event, manifestId: string) => connection?.pointHelm(manifestId));
  handleRepositories({ ipc: ipcMain, connection: () => connection, windowIdOf });
  // A declared server in the main checkout. `servers` on the published state is what keeps a *Serving*
  // row on screen after the sheet that started it closes.
  ipcMain.handle(CHANNELS.startServer, (event, name: string) =>
    connection?.rehearsal.startServer(name, connection.repositories.pickedByWindow.of(windowIdOf(event))),
  );
  ipcMain.handle(CHANNELS.stopServer, (_event, serverId: string) =>
    connection?.rehearsal.stopServer(serverId),
  );
  // The third channel that leaves this machine. The address is checked against
  // what main published for that server before anything is handed to the OS.
  ipcMain.handle(CHANNELS.openServerLink, (_event, serverId: string, url: string) =>
    openServerLink(published, serverId, url),
  );
  // A link in a model's text: the one opener whose address the renderer sends. `links.ts`.
  ipcMain.handle(CHANNELS.openLink, (_event, address: string) => openLink(address));
  // Asks launchd, not Fleet: `restart-fleet.ts`. Read from what main last published.
  ipcMain.handle(CHANNELS.restartFleet, () => restartFleet(published.connection));
  // Asks Fleet, which starts `scripts/restart-build` detached: `fleet-build.ts`.
  ipcMain.handle(CHANNELS.changeFleetBuild, (_event, build: BuildSource, adopt: boolean) =>
    connection?.fleetBuild.change(build, adopt),
  );
  ipcMain.handle(CHANNELS.readSessionCheckOutput, (_event, run: number) => connection?.readSessionCheckOutput(run));
  ipcMain.handle(CHANNELS.stopSessionCheck, (_event, run: number, force: boolean) => connection?.stopSessionCheck(run, force));
  // A retro, and the Lessons listing narrowed to the asking window's pick. Read-only.
  ipcMain.handle(CHANNELS.readRetro, (_event, subject: RetroSubject) => connection?.readRetro(subject));
  ipcMain.handle(CHANNELS.readLessons, (event, state: "open" | "accepted") =>
    connection?.readLessons(
      connection.repositories.pickedByWindow.of(windowIdOf(event)),
      state === "accepted" ? "accepted" : "open",
    ),
  );
  // The guided review of the open items, narrowed to the asking window's pick, and one question about one item.
  ipcMain.handle(CHANNELS.reviewLessons, (event) =>
    connection?.reviewLessons(connection.repositories.pickedByWindow.of(windowIdOf(event))),
  );
  ipcMain.handle(CHANNELS.askLesson, (_event, lessonId: string, question: string, history: AskTurn[]) =>
    connection?.askLesson(lessonId, question, history),
  );
  // What fleet is holding disk for, while somebody is deciding about it. Read-only.
  ipcMain.handle(CHANNELS.readHeld, (_event, want: boolean) =>
    connection?.readHeld(want),
  );
  handleSessions({ ipc: ipcMain, connection: () => connection, windowIdOf, pages: new SessionPages() });
  handleSleep({ ipc: ipcMain, connection: () => connection });
  handleSketchShare({ ipc: ipcMain });
  handleAccounts({ ipc: ipcMain, connection: () => connection });
  ipcMain.handle(CHANNELS.getSessionWindows, () => captureWindows.sessionAddresses());
  handlePlanUsage({ ipc: ipcMain, connection: () => connection });
  handleModelDownshift({ ipc: ipcMain, connection: () => connection });
  handleTimeline({ ipc: ipcMain, connection: () => connection });
  handleClashes({ ipc: ipcMain, connection: () => connection });
  handleReviews({ ipc: ipcMain, connection: () => connection, walks: { show: (shown) => captureWindows.openForReview(shown) } });
  handleTerritory({ ipc: ipcMain, connection: () => connection });
  handleBayHistory({ ipc: ipcMain, connection: () => connection });
  handleUsageSummary({ ipc: ipcMain, connection: () => connection });
  handleFlightControls({ ipc: ipcMain, connection: () => connection });
  handleOnboarding({ ipc: ipcMain, connection: () => connection });
  handleIssues({ ipc: ipcMain, connection: () => connection });
  handleStudios({ ipc: ipcMain, connection: () => connection, published: () => published, captureWindows, summon });
  handleHeldNotes({
    ipc: ipcMain,
    store: heldNotes,
    captureWindows,
    send: async (body) => {
      const sent = await connection?.sessions.sendNotes(body);
      return sent === undefined ? UNSENT : sent.ok ? { ok: true } : sent.outcome;
    },
  });
  // A Studio node's own address, `openPullRequest`'s reason: read here off the
  // Studio main published, never off a string the renderer sent. #1406.
  ipcMain.handle(CHANNELS.openStudioNode, (_event, studioId: string, nodeId: string) =>
    openStudioNode(published, studioId, nodeId),
  );

  createWindow();
  connection.start();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow();
  });
});

// A pulled-down walk holds a slot and a server in Fleet, so a Bridge that quits closes them first, for as long as Fleet takes to answer.
let walksClosed = false;
app.on("before-quit", (event) => {
  const held = walksClosed ? [] : captureWindows.heldWalks();
  if (held.length === 0 || connection === null) return;
  walksClosed = true;
  event.preventDefault();
  const closing = Promise.all(held.map((walk) => connection?.reviews.closeWalk(walk.manifestId, walk.walkId)));
  void Promise.race([closing, new Promise((resolve) => setTimeout(resolve, 4000))]).finally(() => app.quit());
});

app.on("window-all-closed", () => {
  // **On macOS the connection stays up with the window closed.** Fleet is a daemon rather than a
  // subprocess of the window, so Sessions progress either way, and the app is still running; what
  // it costs is one socket.
  //
  // Everywhere else the app quits, which stops the connection on its way out.
  if (process.platform === "darwin") return;
  connection?.stop();
  sessionAttention.close();
  app.quit();
});
