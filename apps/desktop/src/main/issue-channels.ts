// The channels a repository's open issues and its Autopilot travel over. The renderer names a Manifest
// and no port.

import type { IpcMain } from "electron";

import type { AutopilotSettings } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";
import { IssuesHost } from "./issues";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;

export function handleIssues({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.listIssues, (_event, manifestId: unknown, refresh: unknown) =>
    typeof manifestId === "string" && manifestId.trim() !== ""
      ? (connection()?.issues.list(manifestId, refresh === true) ?? unsent)
      : unsent,
  );
  ipc.handle(CHANNELS.getIssue, (_event, manifestId: unknown, reference: unknown) =>
    named(manifestId) && named(reference) ? (connection()?.issues.issue(manifestId, reference) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.getPullRequest, (_event, manifestId: unknown, number: unknown) =>
    named(manifestId) && Number.isInteger(number) ? (connection()?.issues.pullRequest(manifestId, number as number) ?? unsent) : unsent,
  );
  ipc.handle(CHANNELS.refreshPullRequest, (_event, manifestId: unknown, number: unknown) =>
    named(manifestId) && Number.isInteger(number) ? (connection()?.issues.refreshPullRequest(manifestId, number as number) ?? unsent) : unsent,
  );
  handleAutopilot({ ipc, connection });
}

const named = (value: unknown): value is string => typeof value === "string" && value.trim() !== "";

/** Autopilot's acts, each refused here where the renderer named no repository or pick. #2092. */
function handleAutopilot({ ipc, connection }: Hosts): void {
  const of = IssuesHost.autopilotOf;
  const on = (manifestId: unknown, act: (host: IssuesHost, manifestId: string) => Promise<unknown>) => {
    const host = connection()?.issues;
    return named(manifestId) && host ? act(host, manifestId) : unsent;
  };
  ipc.handle(CHANNELS.proposeAutopilot, (_event, manifestId: unknown, sentence: unknown, localNow: unknown) =>
    named(sentence) ? on(manifestId, (host, id) => host.proposeAutopilot(id, sentence, typeof localNow === "string" ? localNow : "")) : unsent,
  );
  ipc.handle(CHANNELS.startAutopilot, (_event, manifestId: unknown, settings: unknown) =>
    typeof settings === "object" && settings !== null
      ? on(manifestId, (host, id) => host.autopilot("POST", of(id), settings as AutopilotSettings))
      : unsent,
  );
  ipc.handle(CHANNELS.updateAutopilot, (_event, manifestId: unknown, settings: unknown) =>
    typeof settings === "object" && settings !== null
      ? on(manifestId, (host, id) => host.autopilot("POST", of(id, "/settings"), settings as AutopilotSettings))
      : unsent,
  );
  ipc.handle(CHANNELS.pauseAutopilot, (_event, manifestId: unknown) => on(manifestId, (host, id) => host.autopilot("POST", of(id, "/pause"))));
  ipc.handle(CHANNELS.resumeAutopilot, (_event, manifestId: unknown) => on(manifestId, (host, id) => host.autopilot("POST", of(id, "/resume"))));
  ipc.handle(CHANNELS.stopAutopilot, (_event, manifestId: unknown) => on(manifestId, (host, id) => host.autopilot("POST", of(id, "/stop"))));
  ipc.handle(CHANNELS.getAutopilot, (_event, manifestId: unknown) => on(manifestId, (host, id) => host.autopilot("GET", of(id))));
  ipc.handle(CHANNELS.listAutopilots, () => connection()?.issues.autopilot("GET", "/autopilots") ?? unsent);
  ipc.handle(CHANNELS.dispatchSetAside, (_event, manifestId: unknown, pickId: unknown) =>
    named(pickId) ? on(manifestId, (host, id) => host.autopilot("POST", of(id, `/picks/${encodeURIComponent(pickId)}/dispatch`), undefined, true)) : unsent,
  );
  ipc.handle(CHANNELS.dismissSetAside, (_event, manifestId: unknown, pickId: unknown) =>
    named(pickId) ? on(manifestId, (host, id) => host.autopilot("POST", of(id, `/picks/${encodeURIComponent(pickId)}/dismiss`))) : unsent,
  );
}
