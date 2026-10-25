// The channels the account travels over. Each is one request to Fleet, and the renderer names no port.
// What crosses from the renderer is not trusted, so a login, a row and a kind are checked before Fleet is asked.

import type { IpcMain } from "electron";
import type { HomeKind, HomeRowAt, HomeSide } from "@armada/protocol";

import { CHANNELS } from "../shared/bridge";
import type { FleetConnection } from "./connection";

type Hosts = { ipc: IpcMain; connection: () => FleetConnection | null };

const unsent = { ok: false, outcome: { ok: false, why: "not_connected" } } as const;
const unread = { ok: false, outcome: { ok: false, why: "not_understood" } } as const;
const id = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value > 0;
const named = (value: unknown): value is string => typeof value === "string" && value !== "";

const KINDS: readonly HomeKind[] = ["skills", "plugins", "global_instructions", "sub_agents", "commands", "mcp_servers", "allowlist", "workflows", "memory", "mods"];
const aKind = (value: unknown): value is HomeKind => (KINDS as readonly unknown[]).includes(value);
const aSide = (value: unknown): value is HomeSide => value === "here" || value === "there";

/** The row as Fleet takes it, rebuilt from the fields it names, or `undefined` where one is not what it should be. */
function rowAt(value: unknown): HomeRowAt | undefined {
  if (typeof value !== "object" || value === null) return undefined;
  const { login, kind, name, repository } = value as Record<string, unknown>;
  if (!named(login) || !aKind(kind) || !named(name) || !(repository === undefined || named(repository))) return undefined;
  return { login, kind, name, ...(repository === undefined ? {} : { repository }) };
}

export function handleAccounts({ ipc, connection }: Hosts): void {
  ipc.handle(CHANNELS.getAccounts, () => connection()?.accounts.get() ?? unsent);
  ipc.handle(CHANNELS.startSignIn, () => connection()?.accounts.startSignIn() ?? unsent);
  ipc.handle(CHANNELS.signOut, (_event, accountId: unknown) => (id(accountId) ? (connection()?.accounts.signOut(accountId) ?? unsent) : unsent));
  ipc.handle(CHANNELS.setRepositoryProfile, (_event, root: unknown, accountId: unknown) =>
    typeof root === "string" && root !== "" && (accountId === null || id(accountId))
      ? (connection()?.accounts.setRepositoryProfile(root, accountId) ?? unsent)
      : unsent,
  );
  ipc.handle(CHANNELS.getSharedHome, (_event, login: unknown) => (named(login) ? (connection()?.accounts.sharedHome(login) ?? unsent) : unread));
  ipc.handle(CHANNELS.syncSharedHome, (_event, login: unknown) => (named(login) ? (connection()?.accounts.syncSharedHome(login) ?? unsent) : unread));
  ipc.handle(CHANNELS.addToSharedHome, (_event, row: unknown) => {
    const at = rowAt(row);
    return at === undefined ? unread : (connection()?.accounts.addToSharedHome(at) ?? unsent);
  });
  ipc.handle(CHANNELS.bringFromSharedHome, (_event, row: unknown) => {
    const at = rowAt(row);
    return at === undefined ? unread : (connection()?.accounts.bringFromSharedHome(at) ?? unsent);
  });
  ipc.handle(CHANNELS.chooseSharedHomeSide, (_event, row: unknown, side: unknown) => {
    const at = rowAt(row);
    return at === undefined || !aSide(side) ? unread : (connection()?.accounts.chooseSharedHomeSide(at, side) ?? unsent);
  });
  ipc.handle(CHANNELS.getHomeKinds, (_event, login: unknown) => (named(login) ? (connection()?.accounts.homeKinds(login) ?? unsent) : unread));
  ipc.handle(CHANNELS.setHomeKind, (_event, login: unknown, kind: unknown, on: unknown) =>
    named(login) && aKind(kind) && typeof on === "boolean" ? (connection()?.accounts.setHomeKind(login, kind, on) ?? unsent) : unread,
  );
  ipc.handle(CHANNELS.setHomeRepository, (_event, login: unknown, owner: unknown, name: unknown) =>
    named(login) && named(owner) && named(name) ? (connection()?.accounts.setHomeRepository(login, owner, name) ?? unsent) : unread,
  );
}
