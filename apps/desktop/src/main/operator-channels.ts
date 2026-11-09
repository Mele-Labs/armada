// The channels the Operator page travels over: one request to Fleet each, and the renderer names no port. What crosses
// from the renderer is not trusted, so the words and the ids are checked to be the strings they are before Fleet is asked.

import type { IpcMain } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = {
  ipc: IpcMain;
  connection: () => FleetConnection | null;
};

const unsent = {
  ok: false,
  outcome: {
    ok: false,
    why: "not_connected",
  },
} as const;
const unread = {
  ok: false,
  outcome: {
    ok: false,
    why: "not_understood",
  },
} as const;

const named = (value: unknown): value is string => typeof value === "string" && value !== "";
/** An optional id: absent, or a name. An empty string is not absent. */
const maybeNamed = (value: unknown): value is string | undefined =>
  value === undefined || named(value);

export function handleOperator({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.readOperatorTalk, () => connection()?.operator.readTalk() ?? unsent);
  ipc.handle(CHANNELS.talkToOperator, (_event, words: unknown, about: unknown) =>
    !named(words) || !maybeNamed(about)
      ? unread
      : (connection()?.operator.talk(words, about) ?? unsent),
  );
  ipc.handle(CHANNELS.moveOnOperatorBoard, (_event, id: unknown, before: unknown) =>
    !named(id) || !maybeNamed(before)
      ? unread
      : (connection()?.operator.move(id, before) ?? unsent),
  );
  ipc.handle(CHANNELS.answerOperatorSuggestion, (_event, id: unknown, choice: unknown) =>
    !named(id) || (choice !== 0 && choice !== 1)
      ? unread
      : (connection()?.operator.answerSuggestion(id, choice) ?? unsent),
  );
}
