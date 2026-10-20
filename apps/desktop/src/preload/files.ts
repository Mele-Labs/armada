// The file panel's members of the preload surface. Each names a channel and takes the panel item that opened the
// file, never a path of its own. Main does not handle these channels yet, so an invoke rejects until it does.

import { ipcRenderer } from "electron";

import type { FilesApi, PanelFileItem } from "../shared/api/files";
import { CHANNELS } from "../shared/bridge";

export const filesPreload: FilesApi = {
  readPanelFile: (item) => ipcRenderer.invoke(CHANNELS.readPanelFile, item),
  savePanelFile: (item, text, version) => ipcRenderer.invoke(CHANNELS.savePanelFile, item, text, version),
  pickPanelFile: () => ipcRenderer.invoke(CHANNELS.pickPanelFile),
  openPanelFileIn: (item, where) => ipcRenderer.invoke(CHANNELS.openPanelFileIn, item, where),
  onPanelFileChanged: (onChanged) => {
    const handler = (_event: unknown, item: PanelFileItem): void => onChanged(item);
    ipcRenderer.on(CHANNELS.panelFileChanged, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.panelFileChanged, handler);
    };
  },
};
