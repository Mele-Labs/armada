// The channels Sessions travel over: starting one, talking in it, answering its ask, tuning, closing,
// opening its thread, reading a picture it carried, and the presses on a pull request it holds.
// Out of `index.ts` for the reason `studio-channels.ts` is. Each is one request to Fleet, and the
// renderer names a session and never a repository or a port.

import type { IpcMain } from "electron";

import type { AnswerSessionAsk, SendSessionMessage, TuneSession } from "@armada/protocol";
import { CHANNELS } from "../shared/bridge";
import type { PullRequestPress } from "../shared/api/sessions";
import type { FleetConnection } from "./connection";

type Hosts = {
  ipc: IpcMain;
  connection: () => FleetConnection | null;
  windowIdOf: (event: Electron.IpcMainInvokeEvent) => number;
};

const PRESSES: readonly PullRequestPress[] = ["read", "ready", "merge", "auto_merge", "review"];
const ANSWERS = ["allow_once", "allow_and_remember", "refuse"];
const MODES = ["ask", "auto", "accept_edits", "plan"];

const text = (value: unknown): value is string => typeof value === "string" && value !== "";
const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export function handleSessions({ ipc, connection, windowIdOf }: Hosts): void {
  ipc.handle(CHANNELS.startSession, (event, title?: string) => {
    const fleet = connection();
    if (fleet === null) return unsent;
    return fleet.startSession(fleet.repositories.pickedByWindow.of(windowIdOf(event)), typeof title === "string" && title !== "" ? title : undefined);
  });
  ipc.handle(CHANNELS.sendSessionMessage, (_event, send: SendSessionMessage) => {
    if (!text(send?.session_id) || typeof send.text !== "string") return unsent;
    return connection()?.sessions.send(send) ?? unsent;
  });
  ipc.handle(CHANNELS.answerSessionAsk, (_event, answer: AnswerSessionAsk) => {
    if (!text(answer?.session_id) || !text(answer.call) || !ANSWERS.includes(answer.answer)) return unsent;
    return connection()?.sessions.answer(answer) ?? unsent;
  });
  ipc.handle(CHANNELS.tuneSession, (_event, tune: TuneSession) => {
    if (!text(tune?.session_id) || !MODES.includes(tune.mode)) return unsent;
    return connection()?.sessions.tune(tune) ?? unsent;
  });
  ipc.handle(CHANNELS.closeSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.end(sessionId) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.watchSession, (_event, sessionId: string) =>
    text(sessionId) ? connection()?.sessions.watch(sessionId) : undefined,
  );
  ipc.handle(CHANNELS.readSessionFile, (_event, sessionId: string, file: string) =>
    text(sessionId) && text(file) ? (connection()?.sessions.file(sessionId, file) ?? { ok: false, outcome: unsent.outcome }) : { ok: false, outcome: unsent.outcome },
  );
  ipc.handle(CHANNELS.pressPullRequest, (_event, sessionId: string, number: number, press: PullRequestPress) =>
    text(sessionId) && Number.isInteger(number) && PRESSES.includes(press) ? (connection()?.sessions.press(sessionId, number, press) ?? unsent) : unsent,
  );
}
