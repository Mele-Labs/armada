// A failure card drawn is heard once by whoever watches, toast or band, and again only when the block shows another failure.

import { buildFailedFailure, FailureBlock, watchFailureShown } from "@armada/shell";
import type { BridgeIdentity } from "@armada/protocol";
import { createRoot } from "react-dom/client";
import { act } from "react";
import { afterEach, expect, test, vi } from "vitest";

import "./styles/index.css";

const BRIDGE: BridgeIdentity = {
  auditPath: null,
  fleetProtocol: null,
};

const failure = (line: string) =>
  buildFailedFailure(
    {
      running: "main",
      line,
    },
    BRIDGE,
  );

let unwatch = () => {};
afterEach(() => {
  unwatch();
});

test("drawing a card is heard once, and a redraw of the same card is not heard again", async () => {
  const heard = vi.fn();
  unwatch = watchFailureShown(heard);
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const draw = () =>
    act(async () => {
      root.render(<FailureBlock failure={failure("one")} onCopied={() => {}} />);
    });
  await draw();
  await draw();
  expect(heard).toHaveBeenCalledTimes(1);
  expect(heard.mock.calls[0]?.[0]).toMatchObject({
    code: "bridge.fleet.build_failed",
  });
  await act(async () => root.unmount());
  host.remove();
});
