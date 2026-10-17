// Hand what a Studio node points at to whatever browses the web on this machine.
//
// **The address is read here, off what main published.** The renderer draws it, so it holds a
// copy; it does not send one. An address is the one argument where a string arriving from a click
// handler would be enough on its own: `shell.openExternal` will hand `file:`, and on some
// platforms worse, to the OS. Nothing composed in the renderer reaches it.
//
// **And what is read is still checked.** The scheme has to be `https:` — the only thing a forge
// writes — so a record carrying anything else is refused by name rather than opened. That is a
// second guard on a value that already came from Fleet, because the cost of being wrong here is
// arbitrary URL handling and the cost of the check is one comparison.
//
// Fleet is not involved, and no protocol version moves.

import { shell } from "electron";

import type { Followed } from "@armada/protocol";
import type { BridgeState } from "../shared/bridge";

/**
 * Whether main will hand this to the OS. `https:` and nothing else.
 *
 * **A scheme test rather than a host allowlist.** Which forges exist is
 * `armada.yml`'s business and Fleet's; what this owns is that Bridge does not
 * become a way to open `file:`, `javascript:` or a registered protocol handler.
 * A malformed string is refused by the parse, which is the same answer.
 */
function addressable(address: string): boolean {
  try {
    return new URL(address).protocol === "https:";
  } catch {
    return false;
  }
}

/** The act that ends with the address, once it is read. */
async function opened(address: string | undefined): Promise<Followed> {
  if (address === undefined) return { ok: false, why: "no_address" };
  if (!addressable(address)) return { ok: false, why: "not_addressable", address };

  try {
    await shell.openExternal(address);
    return { ok: true };
  } catch (error) {
    // The OS explaining itself, carried through rather than replaced with a
    // sentence Bridge made up — `open.ts`'s rule for the same class of failure.
    return { ok: false, why: "refused", address, detail: String(error) };
  }
}

/**
 * The address a Studio node carries, off the Studio main is holding — #1406.
 * `addressOf`'s reason: the renderer draws it and does not send it.
 */
function nodeAddressOf(state: BridgeState, studioId: string, nodeId: string): string | undefined {
  const studio = state.studio;
  if (studio.state !== "read" || studio.studio.id !== studioId) return undefined;
  const node = studio.studio.nodes.find((one) => one.id === nodeId);
  return node !== undefined && "address" in node ? node.address : undefined;
}

/**
 * Open what one Studio node points at — a Link, an Issue, a Pull request or an
 * Epic.
 */
export async function openStudioNode(
  state: BridgeState,
  studioId: string,
  nodeId: string,
): Promise<Followed> {
  return opened(nodeAddressOf(state, studioId, nodeId));
}
