// Saying why a link did not open: a server's, a Studio node's, or one in a model's text.
//
// Each takes the `Followed` main answered and returns the sentence to say, or `null` because it opened: the
// page is in front of the person, which says it better than a toast would. The act itself is handed in as an
// argument, because the socket that opens one belongs to the process that has a filesystem and screens sit above it.

import type { Followed } from "@armada/protocol";

/**
 * Why a server's link did not open, in the app's voice, or `null` because it
 * did.
 *
 * **Not `whyNotFollowed`.** Both answer a `Followed`, but a pull request's
 * sentences are about a job's record and a server's are about a run sheet's —
 * `unknown_job` is a shape neither `openServerLink` nor `stopServer` produces,
 * so it is read here as the same failure `no_address` already covers, rather
 * than given a sentence about a job that was never the question.
 */
export function whyNotOpenedServerLink(followed: Followed): string | null {
  if (followed.ok) return null;
  switch (followed.why) {
    case "unknown_job":
    case "no_address":
      return "This server is no longer on the run sheet's own reading. Reopen it and try again.";
    case "not_addressable":
      return `Fleet resolved ${followed.address} for this server, and Bridge only opens a web address.`;
    case "refused":
      return `This machine did not open ${followed.address}: ${followed.detail}`;
  }
}

/** Asking the host to open one server's link — a server id and its own address. */
export type OpenServerLink = (serverId: string, url: string) => Promise<Followed>;

/**
 * Ask main to open one server's link, and answer with the sentence to say
 * where it did not. `null` is success.
 */
export async function openServerLink(
  open: OpenServerLink,
  serverId: string,
  url: string,
): Promise<string | null> {
  return whyNotOpenedServerLink(await open(serverId, url));
}

/**
 * Why a Studio node's address did not open, in the app's voice, or `null`
 * because it did — #1406.
 *
 * **Not `whyNotFollowed`.** Those sentences are about a Job's record; these are
 * about a node on the board in front of the person, and `unknown_job` is a shape
 * this act cannot produce — a Studio main is not holding answers `no_address`.
 */
export function whyNotOpenedNode(followed: Followed): string | null {
  if (followed.ok) return null;
  switch (followed.why) {
    case "unknown_job":
    case "no_address":
      return "This node carries no address on the Studio Bridge is holding. Reopen the Studio and try again.";
    case "not_addressable":
      return `This node's address is ${followed.address}, and Bridge only opens a web address.`;
    case "refused":
      return `This machine did not open ${followed.address}: ${followed.detail}`;
  }
}

/**
 * Why a link in a model's text did not open, or `null` because it did. Prose
 * only draws `http(s):` as a link, so `not_addressable` is main disagreeing
 * with what the renderer drew. It still gets a sentence.
 */
export function whyNotOpenedLink(followed: Followed): string | null {
  if (followed.ok) return null;
  switch (followed.why) {
    // `main/links.ts` answers neither: it looks nothing up.
    case "unknown_job":
    case "no_address":
      return "Bridge did not open that link.";
    case "not_addressable":
      return `Bridge opens only web addresses, and ${followed.address} is not one.`;
    case "refused":
      return `This machine did not open ${followed.address}: ${followed.detail}`;
  }
}

/** Asking the host to open one Studio node's address — a Studio id and a node id. */
export type OpenStudioNode = (studioId: string, nodeId: string) => Promise<Followed>;

/**
 * Ask main to open what a node points at, and answer with the sentence to say
 * where it did not. `null` is success.
 */
export async function openStudioNode(
  open: OpenStudioNode,
  studioId: string,
  nodeId: string,
): Promise<string | null> {
  return whyNotOpenedNode(await open(studioId, nodeId));
}
