// The Operator page's members of the preload surface. Main answers them from Fleet (`main/operator-channels.ts`); the
// mock answers them in a walk.

import { ipcRenderer } from "electron";

import type { OperatorApi } from "../shared/api/operator";
import { CHANNELS } from "../shared/bridge";

export const operatorPreload: OperatorApi = {
  readOperatorTalk: () => ipcRenderer.invoke(CHANNELS.readOperatorTalk),
  talkToOperator: (words, about) => ipcRenderer.invoke(CHANNELS.talkToOperator, words, about),
  moveOnOperatorBoard: (id, before) => ipcRenderer.invoke(CHANNELS.moveOnOperatorBoard, id, before),
};
