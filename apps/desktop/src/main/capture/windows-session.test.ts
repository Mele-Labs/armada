// A Session's window as `CaptureWindows.openForSession` makes it: the cookie jar is its repository's,
// the origins it may pass through are bare origins, and a sign-in is handed to the window and kept
// nowhere else. `docs/practices/capture-window.md`, *What it may load* and *What the page can reach back into*.

import { beforeEach, expect, it, vi } from "vitest";

vi.mock("electron", () => ({}));

const made = vi.hoisted(() => ({ windows: [] as unknown[][], closed: 0, raised: 0 }));
vi.mock("./window", () => ({
  CaptureWindow: class {
    open = true;
    address: string;
    origins: string[] = [];
    constructor(...args: unknown[]) {
      made.windows.push(args);
      this.address = (args[0] as { url: string }).url;
    }
    showing = () => true;
    passes = (through: readonly string[]) => JSON.stringify(through) === JSON.stringify((made.windows.at(-1)?.[0] as { through: string[] }).through);
    raise = () => void (made.raised += 1);
    close = () => void (made.closed += 1);
    setReadiness = () => {};
    question = () => {};
  },
}));

import type { SessionRecord } from "@armada/protocol";
import { NOTHING_YET } from "../../shared/bridge";
import { partitionFor, type Pinned } from "./address";
import { CaptureWindows } from "./windows";

const AT = "2026-10-15T09:00:00Z";
const PAGE = "https://staging.example.com/app";

const record = (id: string, manifest_id?: string): SessionRecord => ({
  id,
  harness: "h",
  origin: "bridge",
  ...(manifest_id === undefined ? {} : { manifest_id }),
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [],
  waiting_for: [],
});

function windows(...sessions: SessionRecord[]): CaptureWindows {
  const board = { windowsChanged: () => {} } as unknown as ConstructorParameters<typeof CaptureWindows>[0];
  const shown = new CaptureWindows(board);
  shown.changed({ ...NOTHING_YET, sessions: { state: "read", sessions } });
  return shown;
}

const pinOf = (at: number): Pinned => made.windows[at]?.[0] as Pinned;

beforeEach(() => {
  made.windows = [];
  made.closed = 0;
  made.raised = 0;
});

it("keeps a Session's window in its repository's cookie jar, and two repositories apart", () => {
  const shown = windows(record("a", "01ARMADA"), record("b", "01MONOLITH"), record("c"));
  shown.openForSession("a", "One", PAGE);
  shown.openForSession("b", "Two", PAGE);
  shown.openForSession("c", "Three", PAGE);
  expect(pinOf(0).manifestId).toBe("01ARMADA");
  expect(pinOf(1).manifestId).toBe("01MONOLITH");
  expect(pinOf(2).manifestId).toBeNull();
  expect(partitionFor(pinOf(0).manifestId)).not.toBe(partitionFor(pinOf(1).manifestId));
});

it("pins the window to the shown origin and the bare origins named, and drops an entry that is more", () => {
  const shown = windows(record("a", "01ARMADA"));
  shown.openForSession("a", "Staging", PAGE, [], ["https://login.example.org", "https://login.example.org/sso", "ftp://x.example.net"]);
  expect(pinOf(0)).toMatchObject({ origin: "https://staging.example.com", through: ["https://login.example.org"] });
});

it("hands the sign-in to the window and to nothing else", () => {
  const shown = windows(record("a", "01ARMADA"));
  const link = "https://staging.example.com/dev/login?token=abc";
  shown.openForSession("a", "Staging", PAGE, [], [], [link]);
  const [pin, , , , , , signIn] = made.windows[0] ?? [];
  expect(signIn).toEqual([link]);
  expect(JSON.stringify(pin)).not.toContain("token=abc");
});

it("raises the window a Session shows again, unless it comes with a sign-in to run", () => {
  const shown = windows(record("a", "01ARMADA"));
  shown.openForSession("a", "Staging", PAGE);
  shown.openForSession("a", "Staging", PAGE);
  expect([made.windows.length, made.raised, made.closed]).toEqual([1, 1, 0]);
  shown.openForSession("a", "Staging", PAGE, [], [], ["https://staging.example.com/dev/login"]);
  expect([made.windows.length, made.raised, made.closed]).toEqual([2, 1, 1]);
});
