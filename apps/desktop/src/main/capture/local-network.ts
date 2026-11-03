// Armada's own check of what a Job browser's page reaches on this Mac or the person's network, since Electron switches
// Chromium's off and never asks a session for `local-network-access` — `docs/concepts/session/job-browser.md`.

import { BlockList } from "node:net";

import { LOCAL_NETWORK } from "./address";
import type { Decider } from "./permissions";

export type AddressSpace = "loopback" | "local" | "public";

const LOOPBACK = new BlockList();
LOOPBACK.addSubnet("127.0.0.0", 8, "ipv4");
LOOPBACK.addSubnet("0.0.0.0", 8, "ipv4");
LOOPBACK.addAddress("::1", "ipv6");
LOOPBACK.addAddress("::", "ipv6");

const LOCAL = new BlockList();
LOCAL.addSubnet("10.0.0.0", 8, "ipv4");
LOCAL.addSubnet("100.64.0.0", 10, "ipv4");
LOCAL.addSubnet("169.254.0.0", 16, "ipv4");
LOCAL.addSubnet("172.16.0.0", 12, "ipv4");
LOCAL.addSubnet("192.168.0.0", 16, "ipv4");
LOCAL.addSubnet("fc00::", 7, "ipv6");
LOCAL.addSubnet("fe80::", 10, "ipv6");

const RANK: Record<AddressSpace, number> = {
  loopback: 0,
  local: 1,
  public: 2,
};

/** The space of an IP address. */
export function spaceOfAddress(address: string): AddressSpace {
  const family = address.includes(":") ? "ipv6" : "ipv4";
  if (LOOPBACK.check(address, family)) return "loopback";
  return LOCAL.check(address, family) ? "local" : "public";
}

const IP = /^\d+\.\d+\.\d+\.\d+$|:/;

/**
 * The space a host name is in. **A name is looked up**, since one any site can register may answer with 127.0.0.1, and
 * the answer is kept a minute so a page's requests do not each make a lookup. A name that does not resolve is public: the request fails by itself.
 */
export function spacesThrough(
  resolve: (name: string) => Promise<{
    endpoints: {
      address: string;
    }[];
  }>,
  now: () => number = Date.now,
): (host: string) => Promise<AddressSpace> {
  const known = new Map<
    string,
    {
      space: Promise<AddressSpace>;
      until: number;
    }
  >();
  return (host) => {
    const name = host.replace(/^\[|\]$/g, "").toLowerCase();
    if (IP.test(name)) return Promise.resolve(spaceOfAddress(name));
    if (name === "localhost" || name.endsWith(".localhost")) return Promise.resolve("loopback");
    if (name.endsWith(".local")) return Promise.resolve("local");
    const kept = known.get(name);
    if (kept !== undefined && kept.until > now()) return kept.space;
    const space = resolve(name).then(
      ({ endpoints }) =>
        endpoints
          .map(({ address }) => spaceOfAddress(address))
          .reduce<AddressSpace>((most, one) => (RANK[one] < RANK[most] ? one : most), "public"),
      () => "public" as const,
    );
    known.set(name, {
      space,
      until: now() + 60_000,
    });
    return space;
  };
}

type Requested = {
  url: string;
  resourceType: string;
  frame?: {
    origin: string;
    parent: {
      origin: string;
    } | null;
  } | null;
  webContents?: {
    id: number;
    getURL(): string;
  };
};

const REACHED = new Set([
  "http:",
  "https:",
  "ws:",
  "wss:",
]);

function urlOf(address: string): URL | null {
  try {
    const url = new URL(address);
    return REACHED.has(url.protocol) ? url : null;
  } catch {
    return null;
  }
}

/**
 * Who is making a request: an iframe's address is its parent's doing, and a frame with no origin of its own, such as a
 * sandboxed one, answers for its page.
 */
function requesterOf({ frame, resourceType, webContents }: Requested): URL | null {
  const origin =
    resourceType === "subFrame" ? (frame?.parent?.origin ?? frame?.origin) : frame?.origin;
  return urlOf(origin ?? "") ?? urlOf(webContents?.getURL() ?? "");
}

/**
 * Builds the session's test of a request. **Only a Job browser's page is checked, and only a request from a more public
 * address to a more private one**, the rule Chromium's own check has; opening an address in the page itself never is, so
 * the person can always type one. The answer is the row's, and requests waiting on one question share it, since
 * `told.ask` refuses a second question at once.
 */
export function reachTest(
  deciderOf: (contents: Requested["webContents"]) => Decider | undefined,
  spaceOf: (host: string) => Promise<AddressSpace>,
): (request: Requested) => Promise<boolean> {
  const waiting = new Map<string, Promise<boolean>>();
  return async (request) => {
    const own = deciderOf(request.webContents);
    if (own === undefined || request.resourceType === "mainFrame") return true;
    const target = urlOf(request.url);
    const from = requesterOf(request);
    if (target === null || from === null) return true;
    const fromSpace = await spaceOf(from.hostname);
    if (fromSpace === "loopback") return true;
    if (RANK[await spaceOf(target.hostname)] >= RANK[fromSpace]) return true;
    const key = `${request.webContents?.id} ${from.origin}`;
    const kept = waiting.get(key);
    if (kept !== undefined) return kept;
    const answer = own.request(LOCAL_NETWORK, from.origin, {}).finally(() => waiting.delete(key));
    waiting.set(key, answer);
    return answer;
  };
}
