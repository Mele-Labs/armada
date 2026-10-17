// The channels the Reviews tab travels over: one request each to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";
import type { ResolvePullThread, SaveReviewTool, StartReviewRun, WritePullRequest } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const text = (value: unknown): value is string => typeof value === "string" && value !== "";
const count = (value: unknown): value is number => typeof value === "number" && Number.isInteger(value) && value > 0;

export function handleReviews({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getReviewLists, () => connection()?.reviews.lists() ?? unsent);
  ipc.handle(CHANNELS.readReviewPage, (_event, manifestId: string, number: number) =>
    text(manifestId) && count(number) ? (connection()?.reviews.page(manifestId, number) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.readReviewFile, (_event, manifestId: string, number: number, path: string) =>
    text(manifestId) && count(number) && text(path) ? (connection()?.reviews.file(manifestId, number, path) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.writeReview, (_event, manifestId: string, write: WritePullRequest) =>
    text(manifestId) && write !== null && typeof write === "object" ? (connection()?.reviews.write(manifestId, write) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.resolveReviewThread, (_event, manifestId: string, resolve: ResolvePullThread) =>
    text(manifestId) && text(resolve?.thread) && typeof resolve.resolved === "boolean" ? (connection()?.reviews.resolve(manifestId, resolve) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.listReviewTools, (_event, manifestId: string) => (text(manifestId) ? (connection()?.reviews.tools(manifestId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.saveReviewTool, (_event, manifestId: string, save: SaveReviewTool) =>
    text(manifestId) && save !== null && typeof save === "object" ? (connection()?.reviews.saveTool(manifestId, save) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.startReviewRun, (_event, manifestId: string, start: StartReviewRun) =>
    text(manifestId) && count(start?.number) && text(start.name) ? (connection()?.reviews.startRun(manifestId, start) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.getReviewRuns, (_event, manifestId: string, number: number) =>
    text(manifestId) && count(number) ? (connection()?.reviews.runs(manifestId, number) ?? unsent) : unsent,
  );
}
