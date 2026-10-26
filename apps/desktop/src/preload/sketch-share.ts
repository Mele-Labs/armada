// The sketch's one member of the preload surface: save a rendering of it. It names a channel and no path.

import { ipcRenderer } from "electron";

import type { SketchShareApi } from "../shared/api/sketch-share";
import { CHANNELS } from "../shared/bridge";

export const sketchSharePreload: SketchShareApi = {
  saveSketchFile: (files) => ipcRenderer.invoke(CHANNELS.saveSketchFile, files),
  saveDocument: (request) => ipcRenderer.invoke(CHANNELS.saveDocument, request),
};
