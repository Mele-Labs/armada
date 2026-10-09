// The channels Sessions travel over: starting one, talking in it, answering its ask, tuning, closing,
// opening its thread, reading a picture it carried, and the presses on a pull request it holds.
// Out of `index.ts` for the reason `studio-channels.ts` is. Each is one request to Fleet, and the
// renderer names a session and never a repository or a port.

import type { IpcMain } from "electron";

import type { AnswerSessionAsk, AnswerWaiting, RenameSession, SendSessionMessage, TuneSession } from "@armada/protocol";
import { CHANNELS } from "../shared/bridge";
import type { PilotExit, PullRequestPress } from "../shared/api/sessions";
import type { FleetConnection } from "./connection";
import type { SessionPages } from "./session-page";

type Hosts = {
  ipc: IpcMain;
  connection: () => FleetConnection | null;
  windowIdOf: (event: Electron.IpcMainInvokeEvent) => number;
  pages: SessionPages;
};

const PRESSES: readonly PullRequestPress[] = ["read", "ready", "merge", "auto_merge", "review"];
const ANSWERS = ["allow_once", "allow_and_remember", "refuse"];
const MODES = ["ask", "auto", "accept_edits", "plan"];
const OUTCOMES = ["take_over", "restart_step"];
const EXITS: readonly PilotExit[] = ["submit", "attest", "supersede"];

const text = (value: unknown): value is string => typeof value === "string" && value !== "";
const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export function handleSessions({ ipc, connection, windowIdOf, pages }: Hosts): void {
  ipc.handle(CHANNELS.startSession, (event, title?: string, root?: string) => {
    const fleet = connection();
    if (fleet === null) return unsent;
    return fleet.startSession(fleet.repositories.pickedByWindow.of(windowIdOf(event)), typeof title === "string" && title !== "" ? title : undefined, text(root) ? root : undefined);
  });
  ipc.handle(CHANNELS.pilotJob, (_event, jobId: string, outcome: string) =>
    text(jobId) && OUTCOMES.includes(outcome) ? (connection()?.pilotJob(jobId, outcome as "take_over" | "restart_step") ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.exitPilot, (_event, jobId: string, exit: PilotExit, note?: string) =>
    text(jobId) && EXITS.includes(exit) ? (connection()?.pilotExits.exit(jobId, exit, typeof note === "string" && note.trim() !== "" ? note.trim() : undefined) ?? unsent.outcome) : unsent.outcome,
  );
  ipc.handle(CHANNELS.sendSessionMessage, (_event, send: SendSessionMessage) => {
    if (!text(send?.session_id) || typeof send.text !== "string") return unsent;
    return connection()?.sessions.send(send) ?? unsent;
  });
  ipc.handle(CHANNELS.answerSessionAsk, (_event, answer: AnswerSessionAsk) => {
    if (!text(answer?.session_id) || !text(answer.call) || !ANSWERS.includes(answer.answer)) return unsent;
    return connection()?.sessions.answer(answer) ?? unsent;
  });
  ipc.handle(CHANNELS.answerWaiting, (_event, answer: AnswerWaiting) => {
    if (!text(answer?.session_id) || !text(answer.item_id)) return unsent;
    if (answer.mode !== undefined && answer.mode !== "best" && answer.mode !== "quick") return unsent;
    return connection()?.sessions.answerWaiting(answer) ?? unsent;
  });
  ipc.handle(CHANNELS.renameSession, (_event, rename: RenameSession) => {
    if (!text(rename?.session_id) || typeof rename.title !== "string" || rename.title.trim() === "") return unsent;
    return connection()?.sessions.rename({ session_id: rename.session_id, title: rename.title.trim() }) ?? unsent;
  });
  ipc.handle(CHANNELS.tuneSession, (_event, tune: TuneSession) => {
    if (!text(tune?.session_id) || !MODES.includes(tune.mode)) return unsent;
    return connection()?.sessions.tune(tune) ?? unsent;
  });
  ipc.handle(CHANNELS.forkSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.fork(sessionId) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.closeSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.end(sessionId) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.retroSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.retro(sessionId) ?? unsent.outcome) : unsent.outcome,
  );
  ipc.handle(CHANNELS.watchSession, (_event, sessionId: string) =>
    text(sessionId) ? connection()?.sessions.watch(sessionId) : undefined,
  );
  ipc.handle(CHANNELS.openSessionFile, (_event, sessionId: string, path: string) =>
    text(sessionId) && text(path) ? (connection()?.sessions.openFile(sessionId, path) ?? { ok: false, why: "not_addressable", address: path }) : { ok: false, why: "not_addressable", address: "" },
  );
  ipc.handle(CHANNELS.openSessionWindow, (_event, sessionId: string, url: string) =>
    text(sessionId) && text(url) ? (connection()?.sessions.openWindow(sessionId, url) ?? unsent.outcome) : unsent.outcome,
  );
  ipc.handle(CHANNELS.readSessionFile, (_event, sessionId: string, file: string) =>
    text(sessionId) && text(file) ? (connection()?.sessions.file(sessionId, file) ?? { ok: false, outcome: unsent.outcome }) : { ok: false, outcome: unsent.outcome },
  );
  ipc.handle(CHANNELS.readSessionSubagent, (_event, sessionId: string, subagentId: string) =>
    text(sessionId) && text(subagentId) ? (connection()?.sessions.subagent(sessionId, subagentId) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.readSessionArtifact, (_event, sessionId: string, path: string) =>
    text(sessionId) && text(path) ? (connection()?.sessions.readArtifact(sessionId, path) ?? { ok: false, why: "not_addressable" }) : { ok: false, why: "not_addressable" },
  );
  ipc.handle(CHANNELS.showSessionPage, (event, sessionId: string, address: string, bounds: unknown) =>
    text(sessionId) && text(address)
      ? pages.show(event.sender, connection()?.sessions.recordOf(sessionId), address, bounds)
      : { ok: false, why: "not_addressable", address: "" },
  );
  ipc.handle(CHANNELS.moveSessionPage, (event, bounds: unknown) => pages.move(event.sender, bounds));
  ipc.handle(CHANNELS.hideSessionPage, (event) => pages.hide(event.sender));
  ipc.handle(CHANNELS.pressPullRequest, (_event, sessionId: string, number: number, press: PullRequestPress) =>
    text(sessionId) && Number.isInteger(number) && PRESSES.includes(press) ? (connection()?.sessions.press(sessionId, number, press) ?? unsent) : unsent,
  );
}
