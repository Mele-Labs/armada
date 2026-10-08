// A Job's canvas draws its connectors every time it mounts (#1940). React Flow dropped a measured
// node's handle positions when it adopted nodes built before the measurement, and the steps drew with
// no lines in about a quarter of mounts, so the claim is asked on a dozen.

import { expect, test } from "vitest";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

for (let mounted = 1; mounted <= 12; mounted += 1) {
  test(`the approval canvas draws its connectors: mount ${mounted}`, async () => {
    mount("proto/feature-at-approval");
    await onScreen();
    await expect.poll(() => document.querySelectorAll(".react-flow__edge").length, { timeout: 4_000 }).toBeGreaterThan(0);
  });
}
