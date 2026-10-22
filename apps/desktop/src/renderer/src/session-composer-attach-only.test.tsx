// A box that takes files and tags but none of a turn's tuning: no model, effort, mode or sketch, and a file attached still goes with the words.

import { SessionComposer, type SentFromComposer } from "@armada/components";
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

function composer(sent: SentFromComposer[]) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  roots.push({ root, host });
  root.render(
    <SessionComposer
      working={false}
      attachOnly
      model={null}
      effort={null}
      models={["opus"]}
      efforts={["high"]}
      onTune={() => undefined}
      commands={[]}
      taggable={[]}
      tags={[]}
      onTags={() => undefined}
      drawn={[]}
      onDraw={() => undefined}
      onRemoveDrawn={() => undefined}
      onSend={(message) => (sent.push(message), Promise.resolve(true))}
      label="Report"
    />,
  );
}

test("an attach-only box draws no model, effort or sketch", async () => {
  composer([]);
  await expect.element(page.getByRole("button", { name: "Attach file" })).toBeVisible();
  await expect.element(page.getByRole("combobox", { name: "Model" })).not.toBeInTheDocument();
  await expect.element(page.getByRole("combobox", { name: "Effort" })).not.toBeInTheDocument();
  await expect.element(page.getByRole("button", { name: "Draw sketch" })).not.toBeInTheDocument();
});

test("a file attached to an attach-only box goes with the words", async () => {
  const sent: SentFromComposer[] = [];
  composer(sent);
  await userEvent.upload(page.getByLabelText("Files to attach"), new File(["x"], "shot.png", { type: "image/png" }));
  await userEvent.click(page.getByRole("textbox", { name: "Report" }));
  await userEvent.keyboard("it broke{Enter}");
  await expect.poll(() => sent.length).toBe(1);
  expect(sent[0]!.text).toBe("it broke");
  expect(sent[0]!.files.map((one) => one.name)).toEqual(["shot.png"]);
});
