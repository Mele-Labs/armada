// The channels Sessions travel over: starting one, talking in it, answering its ask, tuning, closing,
// opening its thread, reading a picture it carried, and the presses on a pull request it holds.
// Out of `index.ts` for the reason `studio-channels.ts` is. Each is one request to Fleet, and the
// renderer names a session and never a repository or a port.

import type { IpcMain } from "electron";

import type { AnswerSessionAsk, AnswerWaiting, ApprovePlan, ChangeWaiting, ChosenHold, ClaimIssue, ClaimPullRequest, ConfirmWaiting, DismissConcern, DismissWaiting, JudgeSession, RenameSession, SendConcern, SendPlanChanges, SendSessionMessage, SendSessionNote, SettleLater, StartModSession, TuneSession } from "@armada/protocol";
import type { BeginSession, SetNoticeMute, StartAfter, StopWatch } from "@armada/protocol";
import { HELM_CALL_ANSWERS, isNotice } from "@armada/protocol";
import { CHANNELS } from "../shared/bridge";
import type { PullRequestPress, PullRequestWrite, TerminalMovedBack } from "../shared/api/sessions";
import type { FleetConnection } from "./connection";
import type { SessionPages } from "./session-page";
import type { TerminalWindows } from "./terminal-windows";
import type { LoadoutKind, LoadoutSource, McpServerAct } from "@armada/protocol";

type Hosts = {
  ipc: IpcMain;
  connection: () => FleetConnection | null;
  windowIdOf: (event: Electron.IpcMainInvokeEvent) => number;
  pages: SessionPages;
  /** Each Job's terminal window, and where a shell moved back out of one is told: every window. */
  terminals: { windows: Pick<TerminalWindows, "show" | "close" | "sessionIds">; movedBack: (move: TerminalMovedBack) => void };
};

const BACK_TO: readonly TerminalMovedBack["to"][] = ["docked", "side panel"];

const PRESSES: readonly PullRequestPress[] = ["read", "ready", "merge", "auto_merge"];
const WRITES: readonly PullRequestWrite["kind"][] = ["comment", "approve", "request_changes"];
// **From the protocol, not retyped**: a list kept here missed `always_allow` and `always_block`,
// and the card's press came back as Fleet not connected (16 Oct 2026).
const ANSWERS: readonly string[] = HELM_CALL_ANSWERS;
const MODES = ["ask", "auto", "accept_edits", "plan"];
const DELIVERIES = ["auto_land", "open_pr", "keep_local"];
const MCP_ACTS: readonly McpServerAct[] = ["reconnect", "turn_on", "turn_off", "sign_in"];

const text = (value: unknown): value is string => typeof value === "string" && value !== "";
const cells = (value: unknown): value is number => Number.isInteger(value) && (value as number) > 0 && (value as number) <= 65535;
const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
// **A request main cannot read is not a lost connection**: an answer missing from a list here
// was told to the owner as Fleet not connected, while it was (16 Oct 2026).
const unread = { ok: false, outcome: { ok: false, why: "not_understood" } } as const;

type LoadoutItemAsk = { kind: LoadoutKind; name: string; source: LoadoutSource; from?: string };

/** A Loadout row as the window sent it, its four fields text and nothing else; Fleet refuses a kind or a source it does not know. */
function loadoutItemAsk(value: unknown): LoadoutItemAsk | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const { kind, name, source, from } = value as Record<string, unknown>;
  if (!text(kind) || !text(name) || !text(source) || (from !== undefined && !text(from))) return undefined;
  return { kind: kind as LoadoutItemAsk["kind"], name, source: source as LoadoutItemAsk["source"], ...(from === undefined ? {} : { from }) };
}

/** A mute as the window sent it, rebuilt from the fields Fleet reads; anything else is not a choice Fleet could take. */
function noticeMute(value: unknown): SetNoticeMute | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const { notice, scope, muted } = value as Record<string, unknown>;
  if (!isNotice(notice) || typeof muted !== "boolean" || typeof scope !== "object" || scope === null) return undefined;
  const { kind, session_id, manifest_id } = scope as Record<string, unknown>;
  if ((kind === "job" || kind === "hour") && text(session_id)) return { notice, muted, scope: { kind, session_id } };
  if (kind === "repo" && text(manifest_id)) return { notice, muted, scope: { kind, manifest_id } };
  return undefined;
}

export function handleSessions({ ipc, connection, windowIdOf, pages, terminals }: Hosts): void {
  ipc.handle(CHANNELS.startSession, (event, title?: string, root?: string, request?: string, tuning?: { model?: string; effort?: string }, after?: StartAfter, held?: boolean) => {
    const fleet = connection();
    if (fleet === null) return unsent;
    // A model or effort the person did not pick is left off, so Fleet's proposer fills it in.
    const picked = { ...(text(tuning?.model) ? { model: tuning.model } : {}), ...(text(tuning?.effort) ? { effort: tuning.effort } : {}) };
    return fleet.startSession(fleet.repositories.pickedByWindow.of(windowIdOf(event)), typeof title === "string" && title !== "" ? title : undefined, text(root) ? root : undefined, text(request) && request.trim() !== "" ? request : undefined, picked, after !== undefined && text(after.session) ? after : undefined, held === true);
  });
  ipc.handle(CHANNELS.beginSession, (_event, begin: BeginSession) => {
    if (!text(begin?.session_id) || (begin.delivery !== undefined && !DELIVERIES.includes(begin.delivery)) || (begin.message !== undefined && typeof begin.message !== "string")) return unread;
    return connection()?.sessions.begin({ session_id: begin.session_id, ...(begin.delivery === undefined ? {} : { delivery: begin.delivery }), ...(begin.message === undefined ? {} : { message: begin.message }) }) ?? unsent;
  });
  ipc.handle(CHANNELS.readSessionWait, (_event, sessionId: string, request: string) => {
    if (!text(sessionId) || typeof request !== "string") return unread;
    return connection()?.sessions.readWait({ session_id: sessionId, request }) ?? unsent;
  });
  ipc.handle(CHANNELS.changeWaiting, (_event, change: ChangeWaiting) => {
    if (!text(change?.waiting_session) || (change.condition === undefined && change.base === undefined)) return unread;
    return connection()?.sessions.changeWaiting(change) ?? unsent;
  });
  ipc.handle(CHANNELS.startWaitingNow, (_event, sessionId: string) => {
    if (!text(sessionId)) return unread;
    return connection()?.sessions.startWaitingNow({ session_id: sessionId }) ?? unsent;
  });
  ipc.handle(CHANNELS.sendSessionMessage, (_event, send: SendSessionMessage) => {
    if (!text(send?.session_id) || typeof send.text !== "string") return unread;
    return connection()?.sessions.send(send) ?? unsent;
  });
  ipc.handle(CHANNELS.sendSessionNote, (_event, send: SendSessionNote) => {
    if (!text(send?.session_id) || typeof send.said !== "string" || typeof send.address !== "string") return unread;
    if (typeof send.place?.selector !== "string" || typeof send.place.location !== "string" || typeof send.state !== "object" || send.state === null) return unread;
    return connection()?.sessions.sendNote(send) ?? unsent;
  });
  ipc.handle(CHANNELS.startModSession, (_event, start: StartModSession) => {
    const note = start?.note;
    if (typeof note?.said !== "string" || typeof note.address !== "string" || typeof note.place?.selector !== "string" || typeof note.place.location !== "string") return unread;
    if (typeof note.state !== "object" || note.state === null) return unread;
    for (const field of [start.title, start.model, start.effort]) if (field !== undefined && typeof field !== "string") return unread;
    return connection()?.sessions.startMod({ note, ...(text(start.title) ? { title: start.title } : {}), ...(text(start.model) ? { model: start.model } : {}), ...(text(start.effort) ? { effort: start.effort } : {}) }) ?? unsent;
  });
  ipc.handle(CHANNELS.answerSessionAsk,(_event, answer: AnswerSessionAsk) => {
    if (!text(answer?.session_id) || !text(answer.call) || !ANSWERS.includes(answer.answer)) return unread;
    return connection()?.sessions.answer(answer) ?? unsent;
  });
  ipc.handle(CHANNELS.answerWaiting, (_event, answer: AnswerWaiting) => {
    if (!text(answer?.session_id) || !text(answer.item_id)) return unread;
    if (answer.mode !== undefined && answer.mode !== "best" && answer.mode !== "quick") return unread;
    return connection()?.sessions.answerWaiting(answer) ?? unsent;
  });
  ipc.handle(CHANNELS.claimPullRequest, (_event, claim: ClaimPullRequest) => {
    if (!Number.isInteger(claim?.number) || claim.number < 1) return unread;
    if (claim.session_id !== undefined && !text(claim.session_id)) return unread;
    return connection()?.sessions.claimPullRequest({ number: claim.number, ...(claim.session_id === undefined ? {} : { session_id: claim.session_id }) }) ?? unsent;
  });
  ipc.handle(CHANNELS.claimIssue, (_event, claim: ClaimIssue) => {
    if (!text(claim?.issue)) return unread;
    if (claim.session_id !== undefined && !text(claim.session_id)) return unread;
    const pulls = claim.pull_requests;
    if (pulls !== undefined && (!Array.isArray(pulls) || !pulls.every((one) => Number.isInteger(one) && one >= 1))) return unread;
    return connection()?.sessions.claimIssue({ issue: claim.issue, ...(claim.session_id === undefined ? {} : { session_id: claim.session_id }), ...(pulls === undefined ? {} : { pull_requests: pulls }) }) ?? unsent;
  });
  ipc.handle(CHANNELS.settleLater, (_event, settle: SettleLater) => {
    if (!text(settle?.id) || !(["done", "filed", "dropped"] as const).includes(settle.how)) return unread;
    if (settle.session_id !== undefined && !text(settle.session_id)) return unread;
    return connection()?.sessions.settleLater({ id: settle.id, how: settle.how, ...(settle.issue === undefined ? {} : { issue: settle.issue }), ...(settle.reason === undefined ? {} : { reason: settle.reason }), ...(settle.session_id === undefined ? {} : { session_id: settle.session_id }) }) ?? unsent;
  });
  ipc.handle(CHANNELS.stopWatch, (_event, stop: StopWatch) => {
    if (!text(stop?.watch) || (stop.session_id !== undefined && !text(stop.session_id))) return unread;
    return connection()?.sessions.stopWatch({ watch: stop.watch, ...(stop.session_id === undefined ? {} : { session_id: stop.session_id }) }) ?? unsent;
  });
  ipc.handle(CHANNELS.dismissWaiting, (_event, dismiss: DismissWaiting) => {
    if (!text(dismiss?.session_id) || !text(dismiss.item_id)) return unread;
    return connection()?.sessions.dismissWaiting(dismiss) ?? unsent;
  });
  ipc.handle(CHANNELS.confirmWaiting, (_event, confirm: ConfirmWaiting) => {
    if (!text(confirm?.session_id) || !text(confirm.item_id)) return unread;
    return connection()?.sessions.confirmWaiting(confirm) ?? unsent;
  });
  ipc.handle(CHANNELS.approvePlan, (_event, approve: ApprovePlan) => {
    if (!text(approve?.session_id) || !Number.isInteger(approve.version) || approve.version < 1) return unread;
    return connection()?.sessions.approvePlan({ session_id: approve.session_id, version: approve.version }) ?? unsent;
  });
  ipc.handle(CHANNELS.sendPlanChanges, (_event, changes: SendPlanChanges) => {
    if (!text(changes?.session_id) || !Number.isInteger(changes.version) || changes.version < 1 || !Array.isArray(changes.marks)) return unread;
    return connection()?.sessions.sendPlanChanges({ session_id: changes.session_id, version: changes.version, marks: changes.marks, approve: changes.approve === true }) ?? unsent;
  });
  ipc.handle(CHANNELS.judgeSession, (_event, judge: JudgeSession) => {
    if (!text(judge?.session_id)) return unread;
    return connection()?.sessions.judgeSession({ session_id: judge.session_id }) ?? unsent;
  });
  ipc.handle(CHANNELS.sendConcern, (_event, send: SendConcern) => {
    if (!text(send?.session_id) || !text(send.item_id)) return unread;
    return connection()?.sessions.sendConcern({ session_id: send.session_id, item_id: send.item_id }) ?? unsent;
  });
  ipc.handle(CHANNELS.dismissConcern, (_event, dismiss: DismissConcern) => {
    if (!text(dismiss?.session_id) || !text(dismiss.item_id)) return unread;
    return connection()?.sessions.dismissConcern({ session_id: dismiss.session_id, item_id: dismiss.item_id }) ?? unsent;
  });
  ipc.handle(CHANNELS.renameSession, (_event, rename: RenameSession) => {
    if (!text(rename?.session_id) || typeof rename.title !== "string" || rename.title.trim() === "") return unread;
    return connection()?.sessions.rename({ session_id: rename.session_id, title: rename.title.trim() }) ?? unsent;
  });
  ipc.handle(CHANNELS.tuneSession, (_event, tune: TuneSession) => {
    if (!text(tune?.session_id) || !MODES.includes(tune.mode)) return unread;
    return connection()?.sessions.tune(tune) ?? unsent;
  });
  ipc.handle(CHANNELS.runSessionCommand, (_event, sessionId: string, command: string) =>
    text(sessionId) && text(command) ? (connection()?.sessions.runCommand({ session_id: sessionId, command }) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.forkSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.fork(sessionId) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.forkSessionAt, (_event, sessionId: string, at: string) =>
    text(sessionId) && text(at) ? (connection()?.sessions.fork(sessionId, at) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.closeSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.end(sessionId) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.stopSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.stop(sessionId) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.retryHandoff, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.retryHandoff(sessionId) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.pauseSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.pause(sessionId) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.resumeSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.resume(sessionId) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.setSessionAutoLand, (_event, sessionId: string, on: boolean) =>
    text(sessionId) && typeof on === "boolean" ? (connection()?.sessions.setAutoLand(sessionId, on) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.setSessionWorkflow, (_event, sessionId: string, workflow?: string) =>
    text(sessionId) ? (connection()?.sessions.setWorkflow(sessionId, text(workflow) ? workflow : undefined) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.releaseSessionHolds, (_event, sessionId: string, toSessionId?: string, rows?: unknown) => {
    if (!text(sessionId)) return unread;
    // Each row a kind and a target, or none sent; anything else is not a choice Fleet could read.
    const chosen = Array.isArray(rows) && rows.every((one) => text(one?.kind) && text(one?.target)) ? (rows as ChosenHold[]).map(({ kind, target }) => ({ kind, target })) : undefined;
    if (rows !== undefined && chosen === undefined) return unread;
    return (
      connection()?.sessions.releaseHolds({ session_id: sessionId, ...(text(toSessionId) ? { to_session_id: toSessionId } : {}), ...(chosen === undefined ? {} : { rows: chosen }) }) ?? unsent
    );
  });
  ipc.handle(CHANNELS.handToSession, (_event, sessionId: string, group: string) =>
    text(sessionId) && text(group) ? (connection()?.sessions.handToSession({ session_id: sessionId, group }) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.moveSession, (_event, sessionId: string, manifestId: string) =>
    text(sessionId) && text(manifestId) ? (connection()?.sessions.move(sessionId, manifestId) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.muteNotice, (_event, mute: unknown) => {
    const body = noticeMute(mute);
    return body === undefined ? unread : (connection()?.sessions.muteNotice(body) ?? unsent);
  });
  ipc.handle(CHANNELS.retroSession, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.retro(sessionId) ?? unsent.outcome) : unread.outcome,
  );
  ipc.handle(CHANNELS.watchSession, (_event, sessionId: string) =>
    text(sessionId) ? connection()?.sessions.watch(sessionId) : undefined,
  );
  ipc.handle(CHANNELS.openSessionFile, (_event, sessionId: string, path: string) =>
    text(sessionId) && text(path) ? (connection()?.sessions.openFile(sessionId, path) ?? { ok: false, why: "not_addressable", address: path }) : { ok: false, why: "not_addressable", address: "" },
  );
  ipc.handle(CHANNELS.openSessionWindow, (_event, sessionId: string, url: string) =>
    text(sessionId) && text(url) ? (connection()?.sessions.openWindow(sessionId, url) ?? unsent.outcome) : unread.outcome,
  );
  ipc.handle(CHANNELS.readSessionFile, (_event, sessionId: string, file: string) =>
    text(sessionId) && text(file) ? (connection()?.sessions.file(sessionId, file) ?? { ok: false, outcome: unsent.outcome }) : { ok: false, outcome: unread.outcome },
  );
  ipc.handle(CHANNELS.readSessionSubagent, (_event, sessionId: string, subagentId: string) =>
    text(sessionId) && text(subagentId) ? (connection()?.sessions.subagent(sessionId, subagentId) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.readSessionLoadout, (_event, sessionId: string) => (text(sessionId) ? (connection()?.sessions.loadout(sessionId) ?? unsent) : unread));
  ipc.handle(CHANNELS.readAttachmentHistory, (_event, sessionId: string) =>
    text(sessionId) ? (connection()?.sessions.attachmentHistory(sessionId) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.readLoadoutItem, (_event, sessionId: string, item: unknown) => {
    const asked = loadoutItemAsk(item);
    return text(sessionId) && asked !== undefined ? (connection()?.sessions.loadoutItem(sessionId, asked) ?? unsent) : unread;
  });
  ipc.handle(CHANNELS.readSessionMcp, (_event, sessionId: string) => (text(sessionId) ? (connection()?.sessions.mcp(sessionId) ?? unsent) : unread));
  ipc.handle(CHANNELS.actOnSessionMcp, (_event, sessionId: string, server: string, act: McpServerAct) =>
    text(sessionId) && text(server) && MCP_ACTS.includes(act) ? (connection()?.sessions.actOnMcp(sessionId, server, act) ?? unsent) : unread,
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
  ipc.handle(CHANNELS.watchSessionSnapshot, (event, sessionId: string, address: string) =>
    text(sessionId) && text(address)
      ? pages.watch(event.sender, connection()?.sessions.recordOf(sessionId), address)
      : { ok: false, why: "not_addressable", address: "" },
  );
  ipc.handle(CHANNELS.stopSessionSnapshot, (event) => pages.stop(event.sender));
  ipc.handle(CHANNELS.startSessionRun, (_event, sessionId: string, command: string, cols?: number, rows?: number) =>
    text(sessionId) && text(command) && (cols === undefined) === (rows === undefined) && (cols === undefined || (cells(cols) && cells(rows)))
      ? (connection()?.sessions.startRun({ session_id: sessionId, command, ...(cols === undefined || rows === undefined ? {} : { cols, rows }) }) ?? unsent)
      : unread,
  );
  ipc.handle(CHANNELS.openTerminal, (_event, sessionId: string, cols?: number, rows?: number) =>
    text(sessionId) && (cols === undefined) === (rows === undefined) && (cols === undefined || (cells(cols) && cells(rows)))
      ? (connection()?.sessions.openTerminal({ session_id: sessionId, ...(cols === undefined || rows === undefined ? {} : { cols, rows }) }) ?? unsent)
      : unread,
  );
  ipc.handle(CHANNELS.openTerminalWindow, (_event, sessionId: unknown) => {
    if (text(sessionId)) terminals.windows.show(sessionId);
  });
  ipc.handle(CHANNELS.getTerminalWindows, () => terminals.windows.sessionIds());
  ipc.handle(CHANNELS.readTerminalSoFar, (_event, runId: unknown) => (text(runId) ? (connection()?.sessions.soFar(runId) ?? null) : null));
  // The window closes here; the document that draws the Job's dock opens the shell where it went.
  ipc.handle(CHANNELS.moveTerminalBack, (_event, sessionId: unknown, to: unknown) => {
    if (!text(sessionId) || !BACK_TO.includes(to as TerminalMovedBack["to"])) return;
    terminals.windows.close(sessionId);
    const shell = connection()?.sessions.shellNow(sessionId) ?? null;
    terminals.movedBack({ sessionId, to: to as TerminalMovedBack["to"], ...(shell === null ? {} : { shell }) });
  });
  ipc.handle(CHANNELS.catchUpCheckout, (_event, manifestId: string) =>
    text(manifestId) ? (connection()?.sessions.catchUpCheckout({ manifest_id: manifestId }) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.sendCheckoutAsPullRequest, (_event, manifestId: string) =>
    text(manifestId) ? (connection()?.sessions.sendCheckoutAsPullRequest({ manifest_id: manifestId }) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.readCheckoutDiff, (_event, manifestId: string, path: string) =>
    text(manifestId) && text(path) ? (connection()?.sessions.checkoutDiff(manifestId, path) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.stopSessionRun, (_event, runId: string) =>
    text(runId) ? (connection()?.sessions.stopRun({ run_id: runId }) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.writeSessionRun, (_event, runId: string, data: string) =>
    text(runId) && typeof data === "string" ? (connection()?.sessions.writeRun({ run_id: runId, data }) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.resizeSessionRun, (_event, runId: string, cols: number, rows: number) =>
    text(runId) && cells(cols) && cells(rows) ? (connection()?.sessions.resizeRun({ run_id: runId, cols, rows }) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.pressPullRequest, (_event, sessionId: string, number: number, press: PullRequestPress) =>
    text(sessionId) && Number.isInteger(number) && PRESSES.includes(press) ? (connection()?.sessions.press(sessionId, number, press) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.pressRepositoryPullRequest, (_event, manifestId: string, number: number, press: PullRequestPress) =>
    text(manifestId) && Number.isInteger(number) && PRESSES.includes(press) ? (connection()?.sessions.pressInRepository(manifestId, number, press) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.readPullRequestPage, (_event, sessionId: string, number: number) =>
    text(sessionId) && Number.isInteger(number) ? (connection()?.sessions.pullRequestPage(sessionId, number) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.readPullRequestFile, (_event, sessionId: string, number: number, path: string) =>
    text(sessionId) && Number.isInteger(number) && text(path) ? (connection()?.sessions.pullRequestFile(sessionId, number, path) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.writePullRequest, (_event, sessionId: string, number: number, write: PullRequestWrite) =>
    text(sessionId) && Number.isInteger(number) && WRITES.includes(write?.kind) && (write.body === undefined || typeof write.body === "string")
      ? (connection()?.sessions.writePullRequest(sessionId, number, write) ?? unsent)
      : unread,
  );
}
