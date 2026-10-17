// The panel beside a Job's walk window, and Bridge's window following that Job — the owner's call of
// 11 Oct 2026, `docs/practices/capture-window.md`, *The panel beside the page*.
//
// **Main's half is `main/capture/follow.test.ts`**: which windows move Bridge, and that the Job is
// watched and told. Here is what the two documents do with it.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { reviewAtDelivery, running } from "@armada/screens/src/fixtures/build/index";

import "../styles/index.css";
import type { BridgeApi } from "../../../shared/api";
import type { Summons } from "../../../shared/bridge";
import { Panel } from "../walk-panel/Panel";
import { fakeBridge } from "./fake";
import { asRow, holding } from "./holding";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

let unmountPanel = (): void => undefined;
afterEach(() => unmountPanel());

/** The panel's document on its own, as main loads it beside the page, talking to `api`. */
function mountPanel(api: BridgeApi, jobId: string): void {
  window.armada = api;
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <Panel subject={{ kind: "job", id: jobId }} />
    </StrictMode>,
  );
  unmountPanel = () => {
    root.unmount();
    host.remove();
  };
}

test("the panel draws its Job's verdict live: the pull request's Merge and the decision", async () => {
  const fixture = reviewAtDelivery();
  const api = fakeBridge(holding("walk/review", "A Job at its gate, walked", [fixture]));
  // What main's follow does when the window takes focus.
  await api.watchJob(fixture.job.id);
  mountPanel(api, fixture.job.id);
  const panel = page.getByRole("region", { name: `Job: ${fixture.job.handle}` });
  await expect.element(panel.getByRole("button", { name: /^Merge/ }).first()).toBeVisible();
  await expect.element(panel.getByRole("button", { name: "Request changes" })).toBeVisible();
});

test("a panel whose Job Bridge no longer has open says so, and how it comes back", async () => {
  const walked = reviewAtDelivery();
  // Moved whole: every `build/` fixture is the same Job, so renaming its row alone left its read
  // naming the walked Job — and Bridge's "other" Job drew here as this one.
  const other = asRow(running(), 1, "other", "Another Job");
  const api = fakeBridge(holding("walk/elsewhere", "Bridge moved on", [walked, other]));
  await api.watchJob(other.job.id);
  mountPanel(api, walked.job.id);
  await expect
    .element(page.getByText("Bridge has another Job open. This one comes back when this window has focus."))
    .toBeVisible();
});

test("Bridge's window opens the Job a walk window followed, raising nothing else", async () => {
  const fixture = reviewAtDelivery();
  let told: ((to: Summons) => void) | undefined;
  mount({
    ...holding("walk/follow", "A walk window takes focus", [fixture]),
    behaves: () => ({
      onSummoned: (onGo: (to: Summons) => void) => {
        told = onGo;
        return () => undefined;
      },
    }),
  });
  await expect.poll(() => told).toBeDefined();
  told?.({ jobId: fixture.job.id, follow: true });
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
});
