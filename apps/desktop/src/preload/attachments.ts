// The attachment fetch's one member of the preload surface. It names a channel; main decides which addresses it will ask.

import { ipcRenderer } from "electron";

import type { AttachmentsApi } from "../shared/api/attachments";
import { CHANNELS } from "../shared/bridge";

export const attachmentsPreload: AttachmentsApi = {
  fetchAttachment: (url) => ipcRenderer.invoke(CHANNELS.fetchAttachment, url),
};
