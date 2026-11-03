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
const draw = async () => {
  const host = document.createElement("div");
  host.style.cssText = "position:fixed;left:40px;top:30px;width:600px;height:400px;display:flex";
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
  expect(api.place).toHaveBeenLastCalledWith("job", WALK, {
    x: 40,
    y: 30,
    width: 600,
    height: 400,
  });
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
