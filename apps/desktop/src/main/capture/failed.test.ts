import { expect, it } from "vitest";

import { FailedLoads } from "./failed";

const ORIGIN = "http://localhost:5173";
const PAGE = `${ORIGIN}/`;

function watched(): { loads: FailedLoads; told: string[] } {
  const told: string[] = [];
  return { loads: new FailedLoads((url, why) => told.push(`${url} ${why}`)), told };
}

it("tells a refused connection once, however often Reload is pressed, until a page loads", () => {
  const { loads, told } = watched();
  loads.failedLoad(-102, "ERR_CONNECTION_REFUSED", PAGE, true);
  loads.navigated("chrome-error://chromewebdata/", 200, ORIGIN);
  loads.failedLoad(-102, "ERR_CONNECTION_REFUSED", PAGE, true);
  expect(told).toEqual([`${PAGE} ERR_CONNECTION_REFUSED`]);

  loads.navigated(PAGE, 200, ORIGIN);
  loads.failedLoad(-102, "ERR_CONNECTION_REFUSED", PAGE, true);
  expect(told).toHaveLength(2);
});

it("tells an error status on the page itself", () => {
  const { loads, told } = watched();
  loads.navigated(PAGE, 500, ORIGIN);
  expect(told).toEqual([`${PAGE} HTTP 500`]);
});

it("tells a page whose process went away, and not a window closing", () => {
  const { loads, told } = watched();
  loads.gone(PAGE, "clean-exit");
  expect(told).toEqual([]);
  loads.gone(PAGE, "crashed");
  expect(told).toEqual([`${PAGE} the page's process ended: crashed`]);
});

it("tells nothing for a subframe, or a load this window refused", () => {
  const { loads, told } = watched();
  loads.failedLoad(-105, "ERR_NAME_NOT_RESOLVED", "https://ads.example/", false);
  loads.failedLoad(-3, "ERR_ABORTED", "https://elsewhere.example/", true);
  expect(told).toEqual([]);
});
