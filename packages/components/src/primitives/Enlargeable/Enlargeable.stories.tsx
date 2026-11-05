import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn, screen, userEvent, waitFor, within } from "storybook/test";
import { NotableImage } from "../../compositions/ViewerNotes/NotableImage";
import { ViewerNotesFrom } from "../../compositions/ViewerNotes/ViewerNotes";
import { Sheet } from "../Sheet/Sheet";
import { Enlargeable } from "./Enlargeable";

/** A one-pixel picture, so a story never fetches anything. */
export const PIXEL =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

/** A picture in a thread: a thumbnail that opens the same picture over the screen. */
const meta: Meta<typeof Enlargeable> = {
  title: "Primitives/Enlargeable",
  component: Enlargeable,
  args: {
    src: PIXEL,
    name: "login-error.png",
  },
};
export default meta;

type Story = StoryObj<typeof Enlargeable>;

export const Thumbnail: Story = {};

/** A press opens the large view of the same `src`, labelled by the picture's name. */
export const PressOpens: Story = {
  play: async ({ canvas }) => {
    await expect(screen.queryByRole("dialog")).toBeNull();
    await userEvent.click(
      canvas.getByRole("button", {
        name: "login-error.png",
      }),
    );
    const dialog = await screen.findByRole("dialog", {
      name: "login-error.png",
    });
    await expect(
      screen.getByRole("img", {
        name: "login-error.png",
      }),
    ).toHaveAttribute("src", PIXEL);
    await expect(dialog).toBeVisible();
  },
};

/** Enter on the focused thumbnail opens it, so the picture is reachable without a pointer. */
export const EnterOpens: Story = {
  play: async ({ canvas }) => {
    canvas
      .getByRole("button", {
        name: "login-error.png",
      })
      .focus();
    await userEvent.keyboard("{Enter}");
    await screen.findByRole("dialog", {
      name: "login-error.png",
    });
  },
};

/** Escape closes it and focus goes back to the thumbnail that opened it. */
export const EscapeCloses: Story = {
  play: async ({ canvas }) => {
    const thumb = canvas.getByRole("button", {
      name: "login-error.png",
    });
    await userEvent.click(thumb);
    await screen.findByRole("dialog");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await expect(thumb).toHaveFocus();
  },
};

/** A press outside the picture closes it, and focus goes back to the thumbnail. */
export const PressOutsideCloses: Story = {
  play: async ({ canvas }) => {
    const thumb = canvas.getByRole("button", {
      name: "login-error.png",
    });
    await userEvent.click(thumb);
    const dialog = await screen.findByRole("dialog");
    // The scrim is the dialog itself; a press on its own box is a press outside the picture.
    await userEvent.click(dialog);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await expect(thumb).toHaveFocus();
  },
};

/**
 * **A rule about what does not happen.** Escape that closes the picture must not also reach what is
 * behind it, or one press would close the picture and the panel under it.
 */
export const EscapeStopsHere: Story = {
  play: async ({ canvas }) => {
    const behind = fn();
    document.addEventListener("keydown", behind);
    try {
      await userEvent.click(
        canvas.getByRole("button", {
          name: "login-error.png",
        }),
      );
      await screen.findByRole("dialog");
      await userEvent.keyboard("{Escape}");
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
      await expect(behind).not.toHaveBeenCalled();
    } finally {
      document.removeEventListener("keydown", behind);
    }
  },
};

/** Wider than any box a story draws in, so the zoomed picture has somewhere to pan. */
const WIDE =
  "data:image/svg+xml," +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" width="2000" height="1400"><rect width="2000" height="1400" fill="teal"/></svg>',
  );

const scaleOf = (picture: HTMLElement) =>
  Number(/scale\(([\d.]+)\)/.exec(picture.style.transform)?.[1]);
const panOf = (picture: HTMLElement) => {
  const [, x = "", y = ""] = /translate\((-?[\d.]+)px, (-?[\d.]+)px\)/.exec(picture.style.transform) ?? [];
  return {
    x: Number(x),
    y: Number(y),
  };
};

async function opened(canvas: { getByRole: typeof screen.getByRole }) {
  const thumb = canvas.getByRole("button", {
    name: "wide.png",
  });
  await userEvent.click(thumb);
  const dialog = await screen.findByRole("dialog", {
    name: "wide.png",
  });
  const picture = within(dialog).getByRole("img", {
    name: "wide.png",
  });
  await waitFor(() => expect((picture as HTMLImageElement).naturalWidth).toBe(2000));
  return {
    thumb,
    dialog,
    picture,
  };
}

/** A press and its lift at a point: a tap, to the box's own gesture handling. */
function tap(on: HTMLElement, clientX: number, clientY: number) {
  const at = {
    clientX,
    clientY,
    button: 0,
    pointerId: 1,
  };
  fireEvent.pointerDown(on, at);
  fireEvent.pointerUp(on, at);
  fireEvent.click(on, at);
}

const middleOf = (element: HTMLElement) => {
  const box = element.getBoundingClientRect();
  return {
    x: box.left + box.width / 2,
    y: box.top + box.height / 2,
  };
};

const wheel = (on: HTMLElement, deltaY: number, ctrlKey = false) => {
  const { x, y } = middleOf(on);
  const event = new WheelEvent("wheel", {
    deltaY,
    ctrlKey,
    clientX: x,
    clientY: y,
    bubbles: true,
    cancelable: true,
  });
  on.dispatchEvent(event);
  return event;
};

const wideArgs = {
  src: WIDE,
  name: "wide.png",
};

/** **The keyboard contract**: `+` and `=` zoom in, `-` out, `0` back to fit, and the arrows pan only when zoomed. */
export const KeysZoomAndPan: Story = {
  args: wideArgs,
  play: async ({ canvas }) => {
    const { picture } = await opened(canvas);
    await expect(scaleOf(picture)).toBe(1);

    await userEvent.keyboard("{ArrowRight}");
    await expect(panOf(picture).x).toBe(0);

    await userEvent.keyboard("+");
    await expect(scaleOf(picture)).toBeCloseTo(1.5);
    await userEvent.keyboard("=");
    await expect(scaleOf(picture)).toBeCloseTo(2.25);
    await userEvent.keyboard("-");
    await expect(scaleOf(picture)).toBeCloseTo(1.5);

    for (let at = 0; at < 6; at += 1) await userEvent.keyboard("+");
    const start = panOf(picture);
    await userEvent.keyboard("{ArrowRight}");
    await expect(panOf(picture).x).toBe(start.x - 64);
    await userEvent.keyboard("{ArrowLeft}");
    await expect(panOf(picture).x).toBe(start.x);
    await userEvent.keyboard("{ArrowDown}");
    await expect(panOf(picture).y).toBe(start.y - 64);
    await userEvent.keyboard("{ArrowUp}");
    await expect(panOf(picture).y).toBe(start.y);

    await userEvent.keyboard("0");
    await expect(scaleOf(picture)).toBe(1);
    await expect(panOf(picture)).toEqual({
      x: 0,
      y: 0,
    });
  },
};

/** A key that is the app's own, held with a modifier, is left alone. */
export const ModifiedKeysPass: Story = {
  args: wideArgs,
  play: async ({ canvas }) => {
    const { picture } = await opened(canvas);
    await userEvent.keyboard("{Meta>}={/Meta}");
    await expect(scaleOf(picture)).toBe(1);
  },
};

/** **Clamped at both ends**: fit is the floor and 8x the ceiling, by keys and by wheel. */
export const ClampedAtFitAndEight: Story = {
  args: wideArgs,
  play: async ({ canvas }) => {
    const { dialog, picture } = await opened(canvas);
    await userEvent.keyboard("-");
    await expect(scaleOf(picture)).toBe(1);
    for (let at = 0; at < 8; at += 1) await userEvent.keyboard("+");
    await expect(scaleOf(picture)).toBe(8);
    wheel(dialog, -5000);
    await expect(scaleOf(picture)).toBe(8);
    wheel(dialog, 50000);
    await expect(scaleOf(picture)).toBe(1);
  },
};

/** A wheel zooms and the page does not: a pinch left to the browser would zoom the whole window. A pinch is the faster of the two. */
export const WheelZooms: Story = {
  args: wideArgs,
  play: async ({ canvas }) => {
    const { dialog, picture } = await opened(canvas);
    const spun = wheel(dialog, -100);
    await expect(spun.defaultPrevented).toBe(true);
    const byWheel = scaleOf(picture);
    await expect(byWheel).toBeGreaterThan(1);
    await userEvent.keyboard("0");
    wheel(dialog, -100, true);
    await expect(scaleOf(picture)).toBeGreaterThan(byWheel);
  },
};

/** A double press toggles fit and 2.5x. A single one does neither. */
export const DoublePressToggles: Story = {
  args: wideArgs,
  play: async ({ canvas }) => {
    const { picture } = await opened(canvas);
    const { x, y } = middleOf(picture);
    tap(picture, x, y);
    await expect(scaleOf(picture)).toBe(1);
    tap(picture, x, y);
    await expect(scaleOf(picture)).toBe(2.5);
    tap(picture, x, y);
    tap(picture, x, y);
    await expect(scaleOf(picture)).toBe(1);
  },
};

/** A press on the picture is not a press beside it: the viewer stays. */
export const PressOnThePictureStays: Story = {
  args: wideArgs,
  play: async ({ canvas }) => {
    const { dialog, picture } = await opened(canvas);
    const { x, y } = middleOf(picture);
    tap(picture, x, y);
    await expect(dialog).toBeInTheDocument();
    await expect(screen.getByRole("dialog")).toBe(dialog);
  },
};

/** **A press beside the picture closes it at fit only**: zoomed in, the same press loses nothing by doing nothing. */
export const ScrimClosesAtFitOnly: Story = {
  args: wideArgs,
  play: async ({ canvas }) => {
    const { thumb, dialog, picture } = await opened(canvas);
    // Only just zoomed, so the top edge is still beside the picture.
    wheel(dialog, -25);
    await expect(scaleOf(picture)).toBeGreaterThan(1);
    await expect(picture.getBoundingClientRect().top).toBeGreaterThan(0);
    tap(dialog, middleOf(dialog).x, 0);
    await expect(screen.getByRole("dialog")).toBe(dialog);
    await userEvent.keyboard("0");
    tap(dialog, 0, 0);
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await expect(thumb).toHaveFocus();
  },
};

/** Tab stays among the viewer's own buttons, both ways round: the page behind is out of reach. */
export const TabStaysInside: Story = {
  args: wideArgs,
  play: async ({ canvas }) => {
    const { dialog } = await opened(canvas);
    const close = within(dialog).getByRole("button", {
      name: "Close",
    });
    const seen = new Set<Element | null>();
    for (let at = 0; at < 6; at += 1) {
      await userEvent.tab();
      await expect(dialog.contains(document.activeElement)).toBe(true);
      seen.add(document.activeElement);
    }
    await expect(seen.has(close)).toBe(true);
    await expect(seen.size).toBeGreaterThan(1);
    close.focus();
    await userEvent.tab({
      shift: true,
    });
    await expect(dialog.contains(document.activeElement)).toBe(true);
    await expect(document.activeElement).not.toBe(close);
  },
};

const LARGE = (
  <img
    src={PIXEL}
    alt="large one"
    tabIndex={0}
    onClick={(event) => event.stopPropagation()}
    style={{ width: "calc(var(--space-12) * 4)" }}
  />
);

/**
 * **`large` has no gestures**: what stands in for the picture handles its own presses, so no key, wheel or zoom button
 * reaches it. Close, Esc and a press on the scrim still close it, and Tab still stays inside.
 */
export const LargeHasNoGestures: Story = {
  args: {
    large: LARGE,
  },
  play: async ({ canvas }) => {
    const thumb = canvas.getByRole("button", {
      name: "login-error.png",
    });
    await userEvent.click(thumb);
    const dialog = await screen.findByRole("dialog");
    const inside = within(dialog);
    const large = inside.getByRole("img", {
      name: "large one",
    });
    await expect(
      inside.queryByRole("button", {
        name: "Zoom in",
      }),
    ).toBeNull();
    await userEvent.keyboard("+");
    await expect(large.style.transform).toBe("");
    await expect(wheel(dialog, -100).defaultPrevented).toBe(false);
    await expect(large.style.transform).toBe("");

    await userEvent.click(large);
    await expect(screen.getByRole("dialog")).toBe(dialog);

    for (let at = 0; at < 3; at += 1) {
      await userEvent.tab();
      await expect(dialog.contains(document.activeElement)).toBe(true);
    }

    await userEvent.click(
      inside.getByRole("button", {
        name: "Close",
      }),
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
    await expect(thumb).toHaveFocus();

    await userEvent.click(thumb);
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());

    await userEvent.click(thumb);
    await userEvent.click(await screen.findByRole("dialog"));
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  },
};

const closeSheet = fn();

/**
 * **Esc closes what is on top.** The sheet's own listener runs first, since it was added first, and the viewer is portalled
 * out of it; one press closed both before the sheet learnt to look for it.
 */
export const EscapeLeavesTheSheetOpen: Story = {
  render: (args) => (
    <Sheet kind="enlargeable-story" open title="Reading" onClose={closeSheet} closeLabel="Close sheet">
      <Enlargeable {...args} />
    </Sheet>
  ),
  play: async () => {
    const sheet = screen.getByRole("dialog", {
      name: "Reading",
    });
    await userEvent.click(
      within(sheet).getByRole("button", {
        name: "login-error.png",
      }),
    );
    await screen.findByRole("dialog", {
      name: "login-error.png",
    });
    await userEvent.keyboard("{Escape}");
    await waitFor(() =>
      expect(
        screen.queryByRole("dialog", {
          name: "login-error.png",
        }),
      ).toBeNull(),
    );
    await expect(closeSheet).not.toHaveBeenCalled();
    await userEvent.keyboard("{Escape}");
    await expect(closeSheet).toHaveBeenCalledTimes(1);
  },
};

const closeUnder = fn();

/**
 * **A region drawn in the large view goes first, then the view, then the sheet under it**: one Esc each. A press on the
 * picture stays open and draws. Focus stayed on the box before the press took it, so the picture heard no Esc at all.
 */
export const RegionGoesFirstOnEscape: Story = {
  render: () => (
    <ViewerNotesFrom
      host={{
        jobs: [],
        send: () => true,
      }}
    >
      <Sheet kind="enlargeable-story" open title="Reading" onClose={closeUnder} closeLabel="Close sheet">
        <Enlargeable
          src={WIDE}
          name="wide.png"
          large={
            <NotableImage
              className="armada-enlargeable__picture"
              src={WIDE}
              name="wide.png"
              notes={{
                key: "image:story:wide",
                job: "story",
              }}
            />
          }
        />
      </Sheet>
    </ViewerNotesFrom>
  ),
  play: async () => {
    const sheet = screen.getByRole("dialog", {
      name: "Reading",
    });
    await userEvent.click(
      within(sheet).getByRole("button", {
        name: "wide.png",
      }),
    );
    const viewer = await screen.findByRole("dialog", {
      name: "wide.png",
    });
    const picture = within(viewer).getByRole("img", {
      name: "wide.png",
    }) as HTMLImageElement;
    await waitFor(() => expect(picture.naturalWidth).toBe(2000));
    // A synthetic pointer is no pointer the browser can capture.
    picture.setPointerCapture = () => {};
    const box = picture.getBoundingClientRect();
    const from = {
      clientX: box.left + 10,
      clientY: box.top + 10,
      button: 0,
      pointerId: 1,
    };
    const to = {
      ...from,
      clientX: box.left + 110,
      clientY: box.top + 80,
    };
    fireEvent.pointerDown(picture, from);
    fireEvent.pointerMove(picture, to);
    fireEvent.pointerUp(picture, to);
    fireEvent.click(picture, to);
    await expect(screen.getByRole("dialog", { name: "wide.png" })).toBe(viewer);
    const region = () => viewer.querySelector(".armada-notable-image__region[data-drawing]");
    await waitFor(() => expect(region()).not.toBeNull());

    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(region()).toBeNull());
    await expect(screen.getByRole("dialog", { name: "wide.png" })).toBe(viewer);

    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "wide.png" })).toBeNull());
    await expect(closeUnder).not.toHaveBeenCalled();

    await userEvent.keyboard("{Escape}");
    await expect(closeUnder).toHaveBeenCalledTimes(1);
  },
};
