// Autopilot's members of the preload surface. #2092. A file of its own because `index.ts` sits at the
// line count the gate reports; it names channels and no port, like the rest.

import { ipcRenderer } from "electron";
import type { Autopilot, AutopilotSettings } from "@armada/protocol";

import type { AutopilotApi } from "../shared/api/autopilot";
import { CHANNELS } from "../shared/bridge";

export const autopilotPreload: AutopilotApi = {
  proposeAutopilot: (manifestId, sentence, localNow) => ipcRenderer.invoke(CHANNELS.proposeAutopilot, manifestId, sentence, localNow),
  startAutopilot: (manifestId, settings: AutopilotSettings) => ipcRenderer.invoke(CHANNELS.startAutopilot, manifestId, settings),
  pauseAutopilot: (manifestId) => ipcRenderer.invoke(CHANNELS.pauseAutopilot, manifestId),
  resumeAutopilot: (manifestId) => ipcRenderer.invoke(CHANNELS.resumeAutopilot, manifestId),
  stopAutopilot: (manifestId) => ipcRenderer.invoke(CHANNELS.stopAutopilot, manifestId),
  listAutopilots: () => ipcRenderer.invoke(CHANNELS.listAutopilots),
  getAutopilot: (manifestId) => ipcRenderer.invoke(CHANNELS.getAutopilot, manifestId),
  dispatchSetAside: (manifestId, pickId) => ipcRenderer.invoke(CHANNELS.dispatchSetAside, manifestId, pickId),
  dismissSetAside: (manifestId, pickId) => ipcRenderer.invoke(CHANNELS.dismissSetAside, manifestId, pickId),
  onAutopilotChanged: (onChanged) => {
    const handler = (_event: unknown, autopilot: Autopilot): void => onChanged(autopilot);
    ipcRenderer.on(CHANNELS.autopilotChanged, handler);
    return () => void ipcRenderer.removeListener(CHANNELS.autopilotChanged, handler);
  },
};
