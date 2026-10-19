// A pull request's walk as `CaptureWindows.openForReview` makes it: one walk a pull request in the walk window, a web walk on
// the place it was written against, an API walk on a runner of Armada's own, and nothing Fleet publishes about servers ends it.

import { beforeEach, expect, it, vi } from "vitest";

vi.mock("electron", () => ({}));

const made = vi.hoisted(() => ({ windows: [] as unknown[][], closed: 0, raised: 0 }));
vi.mock("./window", () => ({
  CaptureWindow: class {
    open = true;
    address: string;
    walkId: string;
    walkManifestId: string;
    origins: string[] = [];
    constructor(...args: unknown[]) {
      made.windows.push(args);
      this.address = (args[0] as { url: string }).url;
      const review = (args[1] as { review: { walkId: string; manifestId: string } }).review;
      this.walkId = review.walkId;
      this.walkManifestId = review.manifestId;
    }
    raise = () => void (made.raised += 1);
    close = () => {
      made.closed += 1;
      this.open = false;
    };
    dispose = () => this.close();
    setReadiness = () => {};
    question = () => {};
  },
}));
vi.mock("./walk-window", async () => ({ WalkWindow: (await import("./walk-window.fake")).FakeWalkWindow }));

import type { ReviewRequest, WalkStep } from "@armada/protocol";
import { NOTHING_YET } from "../../shared/bridge";
import type { ShownReviewWalk } from "../../shared/review-walk";
import type { Pinned } from "./address";
import { made as walkWindows } from "./walk-window.fake";
import { CaptureWindows } from "./windows";

const steps: WalkStep[] = [{ say: "Open the list", press: { role: "button", name: "Drafts" } }];
const requests: ReviewRequest[] = [{ say: "List", method: "GET", path: "/invoices", look: "A list" }];
const shown = (more: Partial<ShownReviewWalk> = {}): ShownReviewWalk => ({
  walkId: "w1",
  manifestId: "armada",
  number: 12,
  title: "#12 Take a draft out",
  address: "https://pr-12.preview.invalid/app",
  steps,
  ...more,
});

const told = vi.fn();
const board = { reviewClosed: told, focused: () => {} } as unknown as ConstructorParameters<typeof CaptureWindows>[0];
const pinOf = (at: number): Pinned => made.windows[at]?.[0] as Pinned;

beforeEach(() => {
  made.windows = [];
  made.closed = 0;
  made.raised = 0;
  walkWindows.windows = [];
  told.mockClear();
});

it("opens a web walk on the place it was written against, in the repository's own cookie jar", async () => {
  const windows = new CaptureWindows(board);
  expect(await windows.openForReview(shown())).toEqual({ ok: true });
  expect(pinOf(0)).toMatchObject({ url: "https://pr-12.preview.invalid/app", origin: "https://pr-12.preview.invalid", manifestId: "armada", name: "#12 Take a draft out", through: [] });
  expect(made.windows[0]?.[1]).toMatchObject({ review: { walkId: "w1", number: 12 } });
  expect(windows.heldWalks()).toEqual([{ manifestId: "armada", walkId: "w1" }]);
});

it("leaves the same walk where it is, and puts a later one of the pull request in its place, closing the earlier", async () => {
  const windows = new CaptureWindows(board);
  await windows.openForReview(shown());
  await windows.openForReview(shown());
  expect(made.windows).toHaveLength(1);
  expect(walkWindows.windows[0]?.revealed).toEqual([false]);
  await windows.openForReview(shown({ walkId: "w2" }));
  expect(made.closed).toBe(1);
  expect(told).toHaveBeenCalledWith(expect.objectContaining({ walkId: "w1" }));
  expect(made.windows).toHaveLength(2);
  expect(windows.heldWalks()).toEqual([{ manifestId: "armada", walkId: "w2" }]);
});

it("refuses a walk with nothing to play or nowhere to go, and opens nothing", async () => {
  const windows = new CaptureWindows(board);
  expect(await windows.openForReview(shown({ steps: [] }))).toEqual({ ok: false, why: "no_steps" });
  expect(await windows.openForReview(shown({ steps: [{ say: "x" } as unknown as WalkStep] }))).toEqual({ ok: false, why: "no_steps" });
  expect(await windows.openForReview(shown({ address: "ftp://files.invalid/app" }))).toEqual({ ok: false, why: "no_address" });
  expect(made.windows).toHaveLength(0);
});

it("plays an API walk on a runner on loopback, and stops the runner with the window", async () => {
  const windows = new CaptureWindows(board);
  const outcome = await windows.openForReview({ ...shown({ address: "http://localhost:8080/api" }), steps: undefined, requests });
  expect(outcome).toEqual({ ok: true });
  expect(pinOf(0).origin).toMatch(/^http:\/\/127\.0\.0\.1:\d+$/);
  const page = await fetch(pinOf(0).url);
  expect(await page.text()).toContain("Send 1: GET /invoices");
  walkWindows.windows[0]?.close();
  await expect(fetch(pinOf(0).url)).rejects.toThrow();
  expect(told).toHaveBeenCalledWith(expect.objectContaining({ walkId: "w1" }));
});

it("is not ended by what Fleet publishes about servers", async () => {
  const windows = new CaptureWindows(board);
  await windows.openForReview(shown());
  windows.changed({ ...NOTHING_YET, sessions: { state: "read", sessions: [] } });
  expect(made.closed).toBe(0);
  expect(windows.heldWalks()).toHaveLength(1);
});
