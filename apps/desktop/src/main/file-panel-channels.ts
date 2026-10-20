// The file panel's channels: read, save, pick, open in, and the change a watcher sends back.
// Out of `index.ts` for the reason `session-channels.ts` is. The dialog, the editor and Finder are
// handed to `file-panel.ts` here, so its tests never load Electron.

import type { SessionRecord } from "@armada/protocol";
import { BrowserWindow, dialog, shell } from "electron";
import type { IpcMain, WebContents } from "electron";

import { FILES_CHANNELS } from "../shared/api/files";
import type { PanelFileItem, PanelFileOpenedIn, PanelFileSaved } from "../shared/api/files";
import type { BridgeState } from "../shared/bridge";
import { createFilePanel, diskPanel } from "./file-panel";
import { inEditor } from "./open";

type Hosts = {
  ipc: IpcMain;
  recordOf: (sessionId: string) => SessionRecord | undefined;
  published: () => BridgeState;
};

const refused: PanelFileOpenedIn = { ok: false, detail: "Not a file this panel opened" };

export function handleFilePanel({ ipc, recordOf, published }: Hosts): void {
  const panel = createFilePanel({
    disk: diskPanel,
    recordOf,
    settingsPath: () => published().settings?.path,
    pick: async () => {
      const window = BrowserWindow.getFocusedWindow();
      const options = { properties: ["openFile" as const] };
      const chosen = await (window === null ? dialog.showOpenDialog(options) : dialog.showOpenDialog(window, options));
      return chosen.canceled ? null : (chosen.filePaths[0] ?? null);
    },
    inEditor: (path) => inEditor(published(), path),
    showInFolder: (path) => shell.showItemInFolder(path),
    send: (owner, item) => {
      const window = owner as WebContents;
      if (!window.isDestroyed()) window.send(FILES_CHANNELS.panelFileChanged, item);
    },
  });

  // A window that is gone lets go of its watcher; one listener each, however many files it reads.
  const watched = new WeakSet<WebContents>();
  const holding = (sender: WebContents): WebContents => {
    if (!watched.has(sender)) {
      watched.add(sender);
      sender.once("destroyed", () => panel.release(sender));
    }
    return sender;
  };

  ipc.handle(FILES_CHANNELS.readPanelFile, (event, item: PanelFileItem) => panel.read(holding(event.sender), item));
  ipc.handle(
    FILES_CHANNELS.savePanelFile,
    (event, item: PanelFileItem, text: unknown, version: unknown): Promise<PanelFileSaved> | PanelFileSaved =>
      typeof text === "string" && typeof version === "string"
        ? panel.save(item, text, version, event.sender)
        : { state: "failed", detail: "Not understood" },
  );
  ipc.handle(FILES_CHANNELS.pickPanelFile, () => panel.pick());
  ipc.handle(FILES_CHANNELS.openPanelFileIn, (_event, item: PanelFileItem, where: unknown) =>
    where === "editor" || where === "finder" ? panel.openIn(item, where) : refused,
  );
}
