// `features.reduceMotion`: Armada's own Reduce Motion. Each Bridge window is told `prefers-reduced-motion: reduce` through the debugger, so every
// stylesheet and canvas that already asks the Mac follows it, and the Mac's own setting and other apps stay as they are. Off, a window answers as the Mac does.

import { app, BrowserWindow, type WebContents } from "electron";

const COMMAND = "Emulation.setEmulatedMedia";

let reducing = false;
let watching = false;

function hold(contents: WebContents): void {
  if (contents.isDestroyed()) return;
  try {
    if (!contents.debugger.isAttached()) contents.debugger.attach();
    void contents.debugger
      .sendCommand(COMMAND, {
        features: [
          {
            name: "prefers-reduced-motion",
            value: "reduce",
          },
        ],
      })
      .catch(() => undefined);
  } catch {
    // A window with another debugger on it keeps its motion until the next change of the setting.
  }
}

function letGo(contents: WebContents): void {
  if (!contents.isDestroyed() && contents.debugger.isAttached()) contents.debugger.detach();
}

/** Called with the setting each time Fleet's list is read or saved. A window made while it is on is held as it opens. */
export function followReduceMotion(on: boolean): void {
  if (!watching) {
    watching = true;
    app.on("browser-window-created", (_event, window) => {
      if (reducing) hold(window.webContents);
    });
  }
  if (on === reducing) return;
  reducing = on;
  for (const window of BrowserWindow.getAllWindows()) (on ? hold : letGo)(window.webContents);
}
