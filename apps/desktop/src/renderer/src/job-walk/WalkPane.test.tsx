// One walk drawn in whatever holds it. In the mock it is `WalkView` over a fixture page; on a real Bridge it is an empty frame that reports where it is, so main can lay the walk's page and bar over it,
// and that holds still pictures of both while a layer of Bridge's own lies over the frame.

import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";
import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { SessionsFrom } from "../sessions-draft";
import { forgetWalks } from "./store";
import { WalkPane } from "./WalkPane";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

const WALK = "walk:billing";
const job = {
  id: "job",
  title: "Billing",
  turn: {
    state: "idle",
  },
  rows: [],
  attachments: [
    {
      kind: "artifact",
      form: "window",
      id: WALK,
      title: "Billing",
      walk: {
        takes: [
          "Compact",
          "Roomy",
        ],
        steps: [
          {
            say: "Look at the plan card",
            target: "plan",
          },
        ],
      },
    },
  ],
} as unknown as Session;
const jobs = [
  job,
];
const held = {
  get: () => jobs,
  subscribe: () => () => undefined,
} as unknown as SessionsDraft;

const roots: {
  root: Root;
  host: HTMLElement;
}[] = [];
const FRAME = {
  x: 40,
  y: 30,
  width: 600,
  height: 400,
};
const draw = async () => {
  const host = document.createElement("div");
  host.style.cssText = `position:fixed;left:${FRAME.x}px;top:${FRAME.y}px;width:${FRAME.width}px;height:${FRAME.height}px;display:flex`;
  document.body.append(host);
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  await act(async () =>
    root.render(
      <SessionsFrom held={held}>
        <WalkPane jobId="job" walkId={WALK} />
      </SessionsFrom>,
    ),
  );
  return root;
};
const armada = (value: unknown) => {
  (
    window as {
      armada?: unknown;
    }
  ).armada = value;
};

beforeEach(forgetWalks);
afterEach(() => {
  armada(undefined);
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
  document.querySelectorAll(".armada-dropdown-menu__panel").forEach((one) => one.remove());
});

test("in the mock it draws the walk's own view, with its takes and its steps", async () => {
  await draw();
  await expect
    .element(
      page.getByRole("group", {
        name: "Walk Billing",
      }),
    )
    .toBeVisible();
  await expect
    .element(
      page.getByRole("radio", {
        name: "Roomy",
      }),
    )
    .toBeVisible();
});

// A 1 x 1 picture, which is all `<img>` needs to be one.
const PNG = Uint8Array.from(
  atob(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/q842iQAAAABJRU5ErkJggg==",
  ),
  (one) => one.charCodeAt(0),
);

test("on a real Bridge it reports its frame, holds still pictures under a layer of Bridge's own, and lets go when it unmounts", async () => {
  const api = {
    place: vi.fn(async () => undefined),
    unplace: vi.fn(async () => undefined),
    freeze: vi.fn(async () => ({
      page: PNG,
      bar: PNG,
    })),
    thaw: vi.fn(async () => undefined),
  };
  armada({
    jobWalk: api,
  });
  const root = await draw();
  await expect.poll(() => api.place.mock.calls.length).toBeGreaterThan(0);
  expect(api.place).toHaveBeenLastCalledWith("job", WALK, FRAME);
  // Nothing drawn in it but the frame: the page and the bar are main's.
  expect(document.querySelectorAll(".job-walk__live img")).toHaveLength(0);
  expect(api.freeze).not.toHaveBeenCalled();

  const layer = document.createElement("div");
  layer.className = "armada-dropdown-menu__panel";
  document.body.append(layer);
  await expect.poll(() => document.querySelectorAll(".job-walk__live img").length).toBe(2);
  expect(api.freeze).toHaveBeenCalledWith("job", WALK);

  layer.remove();
  await expect.poll(() => document.querySelectorAll(".job-walk__live img").length).toBe(0);
  expect(api.thaw).toHaveBeenCalledWith("job", WALK);

  await act(async () => root.unmount());
  expect(api.unplace).toHaveBeenCalledWith("job", WALK);
});

// A walk drawn twice: as a tab of the Job, and in the review sheet over it. Main holds one pair of views for it, so the pane in front (the sheet's) has them and the tab's yields.
const SHEET_FRAME = {
  x: 200,
  y: 80,
  width: 500,
  height: 300,
};
const api = () => ({
  place: vi.fn(async () => undefined),
  unplace: vi.fn(async () => undefined),
  freeze: vi.fn(async () => ({
    page: PNG,
    bar: PNG,
  })),
  thaw: vi.fn(async () => undefined),
});
/** A pane in a host of its own, in the review sheet when it is `front`: the layer holds the frame it draws. */
const drawPane = async (front: boolean) => {
  const box = front ? SHEET_FRAME : FRAME;
  const host = document.createElement("div");
  host.style.cssText = `position:fixed;left:${box.x}px;top:${box.y}px;width:${box.width}px;height:${box.height}px;display:flex`;
  if (front) host.className = "armada-job-review";
  document.body.append(host);
  const root = createRoot(host);
  roots.push({
    root,
    host,
  });
  await act(async () =>
    root.render(
      <SessionsFrom held={held}>
        <WalkPane jobId="job" walkId={WALK} front={front} />
      </SessionsFrom>,
    ),
  );
  return root;
};
const frames = (count: number) =>
  new Promise<void>((done) => {
    let left = count;
    const next = () => (--left <= 0 ? done() : requestAnimationFrame(next));
    requestAnimationFrame(next);
  });

test("while the sheet's pane holds the walk, the tab's pane places nothing and freezes nothing", async () => {
  const live = api();
  armada({
    jobWalk: live,
  });
  const tab = await drawPane(false);
  await expect.poll(() => live.place.mock.calls.length).toBe(1);
  expect(live.place).toHaveBeenLastCalledWith("job", WALK, FRAME);

  await drawPane(true);
  await expect.poll(() => live.place.mock.calls.length).toBe(2);
  expect(live.place).toHaveBeenLastCalledWith("job", WALK, SHEET_FRAME);
  // The sheet is the ground of its own pane and over the tab's: only the pane in front is a view to leave be, so nothing is held.
  await frames(5);
  expect(live.freeze).not.toHaveBeenCalled();
  // The tab moves under the sheet and says nothing.
  const at = roots[0]!.host;
  at.style.left = `${at.offsetWidth}px`;
  await frames(5);
  expect(live.place).toHaveBeenCalledTimes(2);
  expect(live.unplace).not.toHaveBeenCalled();

  // A menu over the sheet holds the one view the walk has, once.
  const menu = document.createElement("div");
  menu.className = "armada-dropdown-menu__panel";
  document.body.append(menu);
  await expect.poll(() => live.freeze.mock.calls.length).toBe(1);
  await frames(5);
  expect(live.freeze).toHaveBeenCalledTimes(1);
  menu.remove();
  await act(async () => tab.unmount());
});

test("a pane that goes while another draws the walk does not unplace the view the other placed", async () => {
  const live = api();
  armada({
    jobWalk: live,
  });
  await drawPane(false);
  await expect.poll(() => live.place.mock.calls.length).toBe(1);
  const sheet = await drawPane(true);
  await expect.poll(() => live.place.mock.calls.length).toBe(2);

  // The sheet closes: the tab is the only pane again, and places its own frame. The hide between would stop a walk that plays by itself.
  await act(async () => sheet.unmount());
  await expect.poll(() => live.place.mock.calls.length).toBe(3);
  expect(live.place).toHaveBeenLastCalledWith("job", WALK, FRAME);
  expect(live.unplace).not.toHaveBeenCalled();

  // The last pane to go lets go of the views.
  await act(async () => roots[0]!.root.unmount());
  expect(live.unplace).toHaveBeenCalledTimes(1);
  expect(live.unplace).toHaveBeenCalledWith("job", WALK);
});

test("a tab's pane that goes while the sheet's draws the walk leaves the sheet's views placed", async () => {
  const live = api();
  armada({
    jobWalk: live,
  });
  const tab = await drawPane(false);
  await expect.poll(() => live.place.mock.calls.length).toBe(1);
  await drawPane(true);
  await expect.poll(() => live.place.mock.calls.length).toBe(2);

  await act(async () => tab.unmount());
  await frames(5);
  expect(live.unplace).not.toHaveBeenCalled();
  expect(live.place).toHaveBeenCalledTimes(2);
});
