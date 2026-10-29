// A forge attachment fetched for the renderer. **Only https GitHub hosts are asked**, at every hop of a redirect, and
// the `gh` token goes to `github.com` alone: the signed address it redirects to carries its own authority, and a
// token forwarded to a storage host is a token handed away. The token is never logged.

import { execFile } from "node:child_process";
import { existsSync } from "node:fs";
import type { IpcMain } from "electron";

import type { AttachmentFetched } from "../shared/api/attachments";
import { CHANNELS } from "../shared/bridge";

export const MOST_BYTES = 200 * 1024 * 1024;
const MOST_HOPS = 5;
const HOSTS = ["github.com", "githubusercontent.com"];

export function isAllowed(address: string): boolean {
  try {
    const url = new URL(address);
    return url.protocol === "https:" && HOSTS.some((host) => url.hostname === host || url.hostname.endsWith(`.${host}`));
  } catch {
    return false;
  }
}

/** Only the forge's own front door gets the token. */
const takesToken = (address: string): boolean => new URL(address).hostname === "github.com";

let token: Promise<string | null> | undefined;
function ghToken(): Promise<string | null> {
  token ??= new Promise((resolve) => {
    // A packaged app's PATH has no Homebrew in it.
    const gh = ["/opt/homebrew/bin/gh", "/usr/local/bin/gh"].find((path) => existsSync(path)) ?? "gh";
    execFile(gh, ["auth", "token"], { timeout: 5000 }, (error, out) => resolve(error === null && out.trim() !== "" ? out.trim() : null));
  });
  return token;
}

export type Fetcher = (address: string, init: { headers: Record<string, string>; redirect: "manual" }) => Promise<Response>;

export async function fetchAttachment(address: unknown, fetcher: Fetcher = fetch, getToken: () => Promise<string | null> = ghToken): Promise<AttachmentFetched | null> {
  if (typeof address !== "string" || !isAllowed(address)) return null;
  try {
    let at = address;
    for (let hop = 0; hop <= MOST_HOPS; hop++) {
      const headers: Record<string, string> = {};
      const secret = takesToken(at) ? await getToken() : null;
      if (secret !== null) headers.authorization = `token ${secret}`;
      const response = await fetcher(at, { headers, redirect: "manual" });
      if (response.status >= 300 && response.status < 400) {
        const next = response.headers.get("location");
        if (next === null) return null;
        at = new URL(next, at).toString();
        if (!isAllowed(at)) return null;
        continue;
      }
      if (!response.ok) return null;
      if (Number(response.headers.get("content-length") ?? 0) > MOST_BYTES) return null;
      const bytes = new Uint8Array(await response.arrayBuffer());
      if (bytes.byteLength > MOST_BYTES) return null;
      const mime = (response.headers.get("content-type") ?? "").split(";")[0]!.trim().toLowerCase();
      return { bytes, mime: mime === "" ? "application/octet-stream" : mime };
    }
    return null;
  } catch {
    return null;
  }
}

export function handleAttachments({ ipc }: { ipc: IpcMain }): void {
  ipc.handle(CHANNELS.fetchAttachment, (_event, address: unknown) => fetchAttachment(address));
}
