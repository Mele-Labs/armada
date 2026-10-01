// Panning a canvas and letting go, through `App`. The owner's note of 28 Sep
// 2026: *I can only temporarily pan until I release the mouse*.
//
// **The release is not what reset it.** A pan renders nothing of the tab, so
// letting go changes nothing; what put the view back was the next render, and
// on a live Job something renders the tab every second or so. The cause is on
// `FitsTheFrame` in `WorkflowCanvas.tsx`. So each claim below pans and then
// makes the tab render, by a different route.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/** How far the pan goes, in window pixels. Well past anything a click tolerates. */
const DRAGGED = { x: -180, y: -60 };

/**
 * Where React Flow has the canvas, as it writes it. **Off the element rather
 * than out of a hook**: what a person is looking at is what the browser drew.
 */
const viewportOf = (surface: string): string =>
  document.querySelector<HTMLElement>(`${surface} .react-flow__viewport`)?.style.transform ?? "none";

/** Open a destination on the moment the note was pinned on, and pan its canvas. */
async function panned(tab: RegExp, surface: string): Promise<string> {
  await page.viewport(1512, 817);
  mount("arc/executing-concurrent");
  await onScreen();
  await page.getByRole("tab", { name: tab }).click();
  // The pane, which is what React Flow binds its pan to. A drag anywhere else
  // on the canvas is a drag on a card.
  //
  // **Waited for, not read.** The press above renders the tab and React Flow
  // mounts inside that render, so a `querySelector` on the line after it is a
  // race against that render. The wait is right either way; what was actually
  // refusing the merge line was the remembered view above, which left no
  // canvas to find at all.
  await expect.poll(() => document.querySelector(`${surface} .react-flow__pane`)).not.toBeNull();
  const pane = document.querySelector<HTMLElement>(`${surface} .react-flow__pane`);
  expect(pane).not.toBeNull();
  // The first fit has to have landed, or the pan below races it.
  await expect.poll(() => viewportOf(surface)).not.toBe("none");
  const box = pane!.getBoundingClientRect();
  const from = { x: Math.round(box.width / 2), y: Math.round(box.height / 2) };
  await userEvent.dragAndDrop(pane!, pane!, {
    sourcePosition: from,
    targetPosition: { x: from.x + DRAGGED.x, y: from.y + DRAGGED.y },
  });
  // **Settled, not snapshotted.** React Flow fits again whenever its
  // container resizes, and on a busy machine the layout can settle after the
  // drag rather than before it — so the transform read on the line after the
  // drop is one a pending fit is about to overwrite, and every assertion
  // downstream compares against a value that was never the resting one. Two
  // equal reads a frame apart is what makes it the resting one.
  let after = viewportOf(surface);
  await expect
    .poll(() => {
      const now = viewportOf(surface);
      const still = now === after;
      after = now;
      return still;
    })
    .toBe(true);
  expect(after).not.toBe("none");
  return after;
}

test("the Workflow canvas stays where it was panned when a press renders the tab again", async () => {
  const surface = ".armada-workflow-tab .armada-workflow-canvas";
  const after = await panned(/^Workflow/, surface);

  // A press on a step card opens its panel, which is a render of the tab.
  // Nothing about it resizes the canvas: the panel is the app's own, over the
  // work area rather than a column beside the run, so no honest re-fit is owed.
  await page.getByRole("button", { name: /^Implement, running/ }).click();
  await expect.element(page.getByRole("dialog", { name: "Implement" }).first()).toBeVisible();

  expect(viewportOf(surface)).toBe(after);
});

test("the Plan canvas stays where it was panned while the app goes on polling", async () => {
  const surface = ".armada-plan-tab__graph .armada-workflow-canvas";
  const after = await panned(/^Plan/, surface);

  // Nothing is pressed. The window is left alone for longer than the app's own
  // poll, which is what the owner was doing when he wrote the note.
  await new Promise((settle) => setTimeout(settle, 2000));
  expect(viewportOf(surface)).toBe(after);
});
