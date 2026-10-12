// A message Fleet refuses comes back to the box it was typed in, rather than
// being lost with the draft the send cleared.

import { SessionComposer } from "@armada/components";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import "./styles/index.css";

const roots: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    root.unmount();
    host.remove();
  }
});

function composer(went: boolean) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(
    <SessionComposer
      working={false}
      mode="auto"
      onMode={() => undefined}
      model={null}
      effort={null}
      models={[]}
      efforts={[]}
      onTune={() => undefined}
      commands={[]}
      taggable={[]}
      tags={[]}
      onTags={() => undefined}
      drawn={[]}
      onDraw={() => undefined}
      onRemoveDrawn={() => undefined}
      onSend={() => Promise.resolve(went)}
      draftKey={`refused-${String(went)}`}
    />,
  );
}

test("a refused message is put back in the box", async () => {
  composer(false);
  const box = page.getByRole("textbox", { name: "Message" });
  await userEvent.click(box);
  await userEvent.keyboard("the whole paragraph{Enter}");
  await expect.element(box).toHaveTextContent("the whole paragraph");
});

test("a message that went leaves the box empty", async () => {
  composer(true);
  const box = page.getByRole("textbox", { name: "Message" });
  await userEvent.click(box);
  await userEvent.keyboard("sent and gone{Enter}");
  await expect.element(box).toHaveTextContent("");
});
