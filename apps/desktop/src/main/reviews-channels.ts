// The channels the Reviews tab travels over: one request each to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";
import type { ForgetReviewSearch, OpenReviewWalk, ResolvePullThread, SaveReviewTool, SaveWalkPlace, StartReviewRun, StartReviewSearch, WritePullRequest } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { ShownReviewWalk, ShownReviewWalkOutcome } from "../shared/review-walk";
import type { FleetConnection } from "./connection";

/** `walks.show` opens the walk window (`CaptureWindows.openForReview`). */
type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null; walks: { show: (shown: ShownReviewWalk) => Promise<ShownReviewWalkOutcome> } };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const text = (value: unknown): value is string => typeof value === "string" && value !== "";
const count = (value: unknown): value is number => typeof value === "number" && Number.isInteger(value) && value > 0;

export function handleReviews({ ipc, connection, walks }: Hosts): void {
  ipc.handle(CHANNELS.getReviewLists, () => connection()?.reviews.lists() ?? unsent);
  ipc.handle(CHANNELS.readReviewPage, (_event, manifestId: string, number: number) =>
    text(manifestId) && count(number) ? (connection()?.reviews.page(manifestId, number) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.readReviewFile, (_event, manifestId: string, number: number, path: string) =>
    text(manifestId) && count(number) && text(path) ? (connection()?.reviews.file(manifestId, number, path) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.readReviewPatches, (_event, manifestId: string, number: number) =>
    text(manifestId) && count(number) ? (connection()?.reviews.patches(manifestId, number) ?? unsent) : unsent,
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
  ipc.handle(CHANNELS.startReviewSearch, (_event, start: StartReviewSearch) =>
    text(start?.words) ? (connection()?.reviews.startSearch(start) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.forgetReviewSearch, (_event, forget: ForgetReviewSearch) =>
    text(forget?.words) ? (connection()?.reviews.forgetSearch(forget) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.getReviewSearches, () => connection()?.reviews.searches() ?? unsent);
  ipc.handle(CHANNELS.findWalkPlace, (_event, manifestId: string, number: number) =>
    text(manifestId) && count(number) ? (connection()?.reviews.findPlace(manifestId, number) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.listWalkPlaces, () => connection()?.reviews.places() ?? unsent);
  ipc.handle(CHANNELS.saveWalkPlace, (_event, manifestId: string, save: SaveWalkPlace) =>
    text(manifestId) && text(save?.area) && save.place !== null && typeof save.place === "object" ? (connection()?.reviews.savePlace(manifestId, save) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.forgetWalkPlace, (_event, manifestId: string, id: string) =>
    text(manifestId) && text(id) ? (connection()?.reviews.forgetPlace(manifestId, id) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.openReviewWalk, (_event, manifestId: string, open: OpenReviewWalk) =>
    text(manifestId) && count(open?.number) && open.place !== null && typeof open.place === "object" ? (connection()?.reviews.openWalk(manifestId, open) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.closeReviewWalk, (_event, manifestId: string, id: string) =>
    text(manifestId) && text(id) ? (connection()?.reviews.closeWalk(manifestId, id) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.showReviewWalk, (_event, shown: ShownReviewWalk): Promise<ShownReviewWalkOutcome> | ShownReviewWalkOutcome =>
    shown !== null && typeof shown === "object" && text(shown.walkId) && text(shown.manifestId) && count(shown.number) && text(shown.address)
      ? walks.show(shown)
      : { ok: false, why: "no_address" },
  );
}
