import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, screen, userEvent, waitFor } from "storybook/test";
import { Enlargeable } from "./Enlargeable";

/** A one-pixel picture, so a story never fetches anything. */
export const PIXEL =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

/** A picture in a thread: a thumbnail that opens the same picture over the screen. */
const meta: Meta<typeof Enlargeable> = {
  title: "Primitives/Enlargeable",
  component: Enlargeable,
  args: { src: PIXEL, name: "login-error.png" },
};
export default meta;

type Story = StoryObj<typeof Enlargeable>;

export const Thumbnail: Story = {};

/** A press opens the large view of the same `src`, labelled by the picture's name. */
export const PressOpens: Story = {
  play: async ({ canvas }) => {
    await expect(screen.queryByRole("dialog")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "login-error.png" }));
    const dialog = await screen.findByRole("dialog", { name: "login-error.png" });
    await expect(screen.getByRole("img", { name: "login-error.png" })).toHaveAttribute("src", PIXEL);
    await expect(dialog).toBeVisible();
  },
};

/** Enter on the focused thumbnail opens it, so the picture is reachable without a pointer. */
export const EnterOpens: Story = {
  play: async ({ canvas }) => {
    canvas.getByRole("button", { name: "login-error.png" }).focus();
    await userEvent.keyboard("{Enter}");
    await screen.findByRole("dialog", { name: "login-error.png" });
  },
};

/** Escape closes it and focus goes back to the thumbnail that opened it. */
export const EscapeCloses: Story = {
  play: async ({ canvas }) => {
    const thumb = canvas.getByRole("button", { name: "login-error.png" });
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
    const thumb = canvas.getByRole("button", { name: "login-error.png" });
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
      await userEvent.click(canvas.getByRole("button", { name: "login-error.png" }));
      await screen.findByRole("dialog");
      await userEvent.keyboard("{Escape}");
      await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
      await expect(behind).not.toHaveBeenCalled();
    } finally {
      document.removeEventListener("keydown", behind);
    }
  },
};
