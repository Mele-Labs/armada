import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { AttachmentTray } from "./AttachmentTray";

/**
 * What waits to go with a message in the spacious composer, over its words: a picture as itself, a file as a tile
 * with its name and its kind, a sketch as a tile that says Sketch. Each has its own Remove.
 */
const meta: Meta<typeof AttachmentTray> = {
  title: "Compositions/Session composer/Attachment tray",
  component: AttachmentTray,
  args: {
    files: [
      {
        id: "f1",
        name: "shot.png",
        src: "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7",
      },
      {
        id: "f2",
        name: "renderer-console.log",
      },
      {
        id: "f3",
        name: "README",
      },
    ],
    drawn: [
      {
        id: "k1",
        title: "pin the clock",
      },
    ],
    onRemoveFile: fn(),
    onRemoveDrawn: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof AttachmentTray>;

/** The tile is the nearest wrapper of a name that holds its own Remove. */
const tileOf = (name: string): HTMLElement =>
  document.querySelector<HTMLElement>(`button[aria-label="Remove ${name}"]`)!.parentElement!;

/** A picture is drawn as itself, named by its file; a file without one is a tile that says what it is. */
export const WhatIsDrawn: Story = {
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("group", {
        name: "Attached",
      }),
    ).toBeVisible();
    await expect(
      canvas.getByRole("img", {
        name: "shot.png",
      }),
    ).toHaveAttribute("src", expect.stringContaining("data:image/gif"));
    await expect(tileOf("renderer-console.log")).toHaveTextContent(/^renderer-console\.log\s*LOG$/);
    await expect(tileOf("pin the clock")).toHaveTextContent(/^pin the clock\s*Sketch$/);
  },
};

/** A name with no dot has no kind to say. */
export const NoDotNoKind: Story = {
  play: async () => {
    await expect(tileOf("README").textContent).toBe("README");
  },
};

/** Each Remove is named for what it takes off and answers with the id of that one, a file or a sketch. */
export const RemovesTheRightOne: Story = {
  play: async ({ canvas, args }) => {
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Remove shot.png",
      }),
    );
    await expect(args.onRemoveFile).toHaveBeenLastCalledWith("f1");
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Remove renderer-console.log",
      }),
    );
    await expect(args.onRemoveFile).toHaveBeenLastCalledWith("f2");
    await expect(args.onRemoveDrawn).not.toHaveBeenCalled();
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Remove pin the clock",
      }),
    );
    await expect(args.onRemoveDrawn).toHaveBeenCalledTimes(1);
    await expect(args.onRemoveDrawn).toHaveBeenCalledWith("k1");
    await expect(args.onRemoveFile).toHaveBeenCalledTimes(2);
  },
};
