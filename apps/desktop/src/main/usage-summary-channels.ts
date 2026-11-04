// The channel the usage summary travels over: one request to Fleet, and the renderer names no port.

import type { IpcMain } from "electron";

import type { UsageWindow } from "../shared/api/usage-summary";
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

/** An offset in whole minutes within a day either way: what crosses from the renderer is not trusted. `null` for anything else. */
export function offsetOf(minutes: unknown): number | null {
  return typeof minutes === "number" && Number.isInteger(minutes) && Math.abs(minutes) <= 24 * 60
    ? minutes
    : null;
}

/** Two instants with the first before the second, as Fleet will read them. `undefined` where none was asked, `null` for anything else. */
export function windowOf(window: unknown): UsageWindow | undefined | null {
  if (window === undefined) return undefined;
  if (typeof window !== "object" || window === null) return null;
  const { since, until } = window as {
    since?: unknown;
    until?: unknown;
  };
  if (typeof since !== "string" || typeof until !== "string") return null;
  const from = Date.parse(since);
  const to = Date.parse(until);
  return Number.isFinite(from) && Number.isFinite(to) && from < to
    ? {
        since,
        until,
      }
    : null;
}

export function handleUsageSummary({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getUsageSummary, (_event, minutes: unknown, window?: unknown) => {
    const offset = offsetOf(minutes);
    const asked = windowOf(window);
    return offset === null || asked === null
      ? unsent
      : (connection()?.usageSummary.get(offset, asked) ?? unsent);
  });
  ipc.handle(CHANNELS.getUsageRules, () => connection()?.usageSummary.rules() ?? unsent);
}
