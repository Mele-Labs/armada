// The channel the Cockpit's timeline travels over: one request to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";
import type { TimelineAsk } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

/** Only the three fields a window has, each the type it is: what crosses from the renderer is not trusted. */
export function windowOf(asked: unknown): TimelineAsk {
  const one = typeof asked === "object" && asked !== null ? (asked as Record<string, unknown>) : {};
  return {
    ...(typeof one.since === "string" ? { since: one.since } : {}),
    ...(typeof one.until === "string" ? { until: one.until } : {}),
    ...(typeof one.limit === "number" && Number.isInteger(one.limit) && one.limit > 0 ? { limit: one.limit } : {}),
  };
}

export function handleTimeline({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getFleetTimeline, (_event, asked: unknown) => connection()?.timeline.get(windowOf(asked)) ?? unsent);
}
