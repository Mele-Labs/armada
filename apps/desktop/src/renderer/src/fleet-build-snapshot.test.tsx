// The window's side of trying a Job in the app: the chime when the snapshot turns ready, the hold while a swap runs,
// the bar over the window while Jobs are tried, and what a refused pick says on the row.

import { createRoot, type Root } from "react-dom/client";
import { act } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { FleetBuildReport, Outcome } from "@armada/protocol";

import "./styles/index.css";

const { played } = vi.hoisted(() => ({
  played: vi.fn(),
}));
vi.mock("./ui-sounds", () => ({
  playUiSound: played,
}));

import { pickRefusedSaid, useSnapshotChime } from "./fleet-build";
import { SwapHold } from "./swap-hold";
import { TryingBar } from "./trying-bar";

(
  globalThis as {
    IS_REACT_ACT_ENVIRONMENT?: boolean;
  }
).IS_REACT_ACT_ENVIRONMENT = true;

let mounted: {
  root: Root;
  host: HTMLElement;
}[] = [];

afterEach(() => {
  for (const one of mounted) {
    act(() => one.root.unmount());
    one.host.remove();
  }
  mounted = [];
});
beforeEach(() => played.mockClear());

function mount(node: React.ReactNode): (next: React.ReactNode) => void {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  mounted.push({
    root,
    host,
  });
  act(() => root.render(node));
  return (next) => act(() => root.render(next));
}

const AT_MAIN: FleetBuildReport = {
  on: "main",
};
const building = (state: "building" | "ready"): FleetBuildReport => ({
  ...AT_MAIN,
  snapshot: {
    state,
  },
});

function Chimed({ report }: { report: FleetBuildReport | null }) {
  useSnapshotChime(report);
  return null;
}

test("the chime plays once, on the move from not ready to ready", () => {
  const again = mount(<Chimed report={AT_MAIN} />);
  again(<Chimed report={building("building")} />);
  expect(played).not.toHaveBeenCalled();
  again(<Chimed report={building("ready")} />);
  expect(played).toHaveBeenCalledTimes(1);
  // The same state read again, and Fleet not answering, are not news.
  again(
    <Chimed
      report={{
        ...building("ready"),
      }}
    />,
  );
  again(<Chimed report={null} />);
  again(<Chimed report={building("ready")} />);
  expect(played).toHaveBeenCalledTimes(1);
});

test("a snapshot already ready when the window opens does not chime", () => {
  mount(<Chimed report={building("ready")} />);
  expect(played).not.toHaveBeenCalled();
});

test("a snapshot rebuilt after the window opened chimes when it turns ready", () => {
  const again = mount(<Chimed report={building("ready")} />);
  again(<Chimed report={building("building")} />);
  again(<Chimed report={building("ready")} />);
  expect(played).toHaveBeenCalledTimes(1);
});

test("a swap holds the window with one banner, and keeps what was typed where it was", async () => {
  const swapping: FleetBuildReport = {
    ...AT_MAIN,
    swapping: true,
  };
  const frame = (report: FleetBuildReport) => (
    <>
      <input aria-label="Draft" />
      <SwapHold report={report} />
    </>
  );
  const again = mount(frame(AT_MAIN));
  const box = document.querySelector("input");
  if (box === null) throw new Error("the box is drawn");
  box.value = "half a thought";
  again(frame(swapping));
  await expect.element(page.getByText("Swapping to the build")).toBeVisible();
  expect(document.querySelectorAll('[role="status"]')).toHaveLength(1);
  expect(document.querySelector('[data-testid="swap-hold"]')).not.toBeNull();
  // Keys are held with the clicks.
  const key = new KeyboardEvent("keydown", {
    key: "a",
    bubbles: true,
    cancelable: true,
  });
  document.body.dispatchEvent(key);
  expect(key.defaultPrevented).toBe(true);
  again(frame(AT_MAIN));
  expect(document.querySelector('[data-testid="swap-hold"]')).toBeNull();
  expect(document.querySelector("input")).toBe(box);
  expect(box.value).toBe("half a thought");
});

test("a swap that was undone says so until it is put away, and a report without the line forgets it", async () => {
  const swappedBack: FleetBuildReport = {
    ...AT_MAIN,
    failed: "Swapped back: the new Fleet did not answer in a minute",
  };
  const again = mount(<SwapHold report={swappedBack} />);
  await expect.element(page.getByText("Swapped back")).toBeVisible();
  await expect.element(page.getByText("the new Fleet did not answer in a minute")).toBeVisible();
  await page
    .getByRole("button", {
      name: "Dismiss",
    })
    .click();
  expect(document.body.textContent).not.toContain("Swapped back");
  // The same line read again stays away; the next report with none forgets it, so a later line is said.
  again(
    <SwapHold
      report={{
        ...swappedBack,
      }}
    />,
  );
  expect(document.body.textContent).not.toContain("Swapped back");
  again(<SwapHold report={AT_MAIN} />);
  again(<SwapHold report={swappedBack} />);
  expect(document.body.textContent).toContain("Swapped back");
});

test("a failure that is not a swap back draws no banner", () => {
  mount(
    <SwapHold
      report={{
        ...AT_MAIN,
        failed: "main cannot fast-forward",
      }}
    />,
  );
  expect(document.querySelector(".armada-swap-hold__banner")).toBeNull();
});

test("the trying bar names the titles, and Keep and Revert are its two acts", async () => {
  const keep = vi.fn();
  const revert = vi.fn();
  mount(
    <TryingBar
      trying={[
        "One",
        "armada/two",
      ]}
      onKeep={keep}
      onRevert={revert}
    />,
  );
  await expect.element(page.getByText("Trying One, armada/two")).toBeVisible();
  await page
    .getByRole("button", {
      name: "Keep",
    })
    .click();
  expect(keep).toHaveBeenCalledTimes(1);
  expect(revert).not.toHaveBeenCalled();
  await page
    .getByRole("button", {
      name: "Revert",
    })
    .click();
  expect(revert).toHaveBeenCalledTimes(1);
});

test("the trying bar is not drawn when nothing is tried", () => {
  mount(<TryingBar trying={[]} onKeep={() => undefined} onRevert={() => undefined} />);
  expect(document.querySelector(".armada-trying-bar")).toBeNull();
});

const refused = (code: string, message: string): Outcome => ({
  ok: false,
  why: "refused",
  error: {
    code,
    message,
    run_id: "r",
    fields: {},
    chain: [],
  },
});

test("a pick on a Job with no branch says so on the row, and any other refusal is the toast", () => {
  const told = vi.fn();
  const raised = vi.fn();
  window.addEventListener("armada:tell", (event) => told((event as CustomEvent).detail.sentence));
  window.addEventListener("armada:raise-outcome", (event) => raised((event as CustomEvent).detail));
  pickRefusedSaid(refused("fleet.build_no_branch", "x"));
  expect(told).toHaveBeenCalledWith("This Job holds no branch yet");
  expect(raised).not.toHaveBeenCalled();
  const other = refused("fleet.build_snapshotting", "A snapshot is building");
  pickRefusedSaid(other);
  expect(raised).toHaveBeenCalledWith(other);
  pickRefusedSaid({
    ok: true,
  });
  expect(told).toHaveBeenCalledTimes(1);
});
