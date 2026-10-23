// Any file in the side panel: read it, save it, pick one, hand it to another program. A slice imports
// protocol, never another slice; `../api.ts` and `../bridge.ts` compose them.
//
// **Main is not written yet.** The renderer is built against these types and a mock answers them.
// A file is named by the panel item that opened it, never by a bare path the renderer chose: main decides
// whether the item is one it may read (`not_addressable`), as it does for a Session's artifact.

/** Where a file was opened from. `settings` is settings.json, `picked` a file the person chose in ⌘O's dialog. */
export type PanelFileFrom = "session" | "settings" | "kit" | "picked";

/** What a press hands over to open a file. `sessionId` is the Session whose files `path` is among, where `from` is `session`. */
export type PanelFileItem = { kind: "file"; path: string; from: PanelFileFrom; sessionId?: string };

/** The file as it is on disk. `version` is whatever main needs to tell later that it changed: a save hands it back. */
export type PanelFileRead =
  | { ok: true; text?: string; bytes?: Uint8Array; type: string; version: string }
  | { ok: false; why: "not_addressable" | "too_big" | "binary" | "unreadable" };

/** What a save came to. `moved` is a file that differs from the one read: nothing was written, and `onDisk` is what is there now. */
export type PanelFileSaved =
  | { state: "saved"; version: string }
  | { state: "moved"; onDisk: string; version: string }
  | { state: "failed"; detail: string };

export type PanelFileOpenedIn = { ok: true } | { ok: false; detail: string };

export type FilesApi = {
  readPanelFile: (item: PanelFileItem) => Promise<PanelFileRead>;
  /** Writes `text` if the file is still at `version`. Saving over a `moved` answer passes the `version` it returned. */
  savePanelFile: (item: PanelFileItem, text: string, version: string) => Promise<PanelFileSaved>;
  /** Main opens a native open dialog. Null where it was closed without a file. */
  pickPanelFile: () => Promise<PanelFileItem | null>;
  /** Text typed into the palette, read as a path: `~` expanded, a relative one resolved in the Session's checkout. Null where it is not a readable file. */
  openPanelPath: (text: string, sessionId?: string) => Promise<PanelFileItem | null>;
  /** The file in the editor the machine opens it with, or selected in Finder. */
  openPanelFileIn: (item: PanelFileItem, where: "editor" | "finder") => Promise<PanelFileOpenedIn>;
  /** Of the paths a Session's words name, those that are files in its checkout, as written. Asked once per path, not per draw. */
  findPanelFiles: (sessionId: string, paths: readonly string[]) => Promise<string[]>;
  /** A file open in the panel changed on disk. The renderer reloads silently when it holds no edit. */
  onPanelFileChanged: (onChanged: (item: PanelFileItem) => void) => () => void;
};

export const FILES_CHANNELS = {
  readPanelFile: "bridge:read-panel-file",
  savePanelFile: "bridge:save-panel-file",
  pickPanelFile: "bridge:pick-panel-file",
  openPanelPath: "bridge:open-panel-path",
  openPanelFileIn: "bridge:open-panel-file-in",
  findPanelFiles: "bridge:find-panel-files",
  panelFileChanged: "bridge:panel-file-changed",
} as const;
