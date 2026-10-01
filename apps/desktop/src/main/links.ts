// Open a link written in a model's text, in whatever browses the web here.
//
// **The one opener that takes the address from the renderer.** `forge.ts` and
// `servers.ts` read theirs off what main published. A link in a Judge's prose
// has no id main could look it up by, and the owner chose clickable links in
// that text, so the scheme check below is the whole guard.

import { shell } from "electron";

import type { Followed } from "@armada/protocol";

/** `http:` and `https:` and nothing else: never `file:`, `javascript:` or a registered handler. */
function addressable(address: string): boolean {
  try {
    const scheme = new URL(address).protocol;
    return scheme === "http:" || scheme === "https:";
  } catch {
    return false;
  }
}

export async function openLink(address: string): Promise<Followed> {
  if (!addressable(address)) return { ok: false, why: "not_addressable", address };
  try {
    await shell.openExternal(address);
    return { ok: true };
  } catch (error) {
    return { ok: false, why: "refused", address, detail: String(error) };
  }
}
