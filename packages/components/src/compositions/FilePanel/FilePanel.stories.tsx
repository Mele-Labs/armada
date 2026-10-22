import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, waitFor, within } from "storybook/test";

import { typeIntoEditor } from "./CodeEditor";
import { FilePanel } from "./FilePanel";

/**
 * Any file in the side panel: a head with its name, its path and two acts that leave Bridge for it, and the body the file fills.
 * Text is edited in place and a picture is only shown.
 *
 * **Every state below is one the file panel's api can produce.** `readPanelFile` answers text or bytes, `savePanelFile`
 * answers saved, moved with what is on disk, or failed, and a read it refuses leaves the head and one line saying why.
 * The panel is a floating sheet, so each story draws it over the canvas.
 */
const meta: Meta<typeof FilePanel> = {
  title: "Compositions/File panel",
  component: FilePanel,
  args: {
    name: "settings.json",
    path: "/Users/user/Library/Application Support/Armada/settings.json",
    type: "text/json",
    dirty: false,
    onClose: fn(),
    onChange: fn(),
    onSave: fn(),
    onOpenInEditor: fn(),
    onShowInFinder: fn(),
  },
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof FilePanel>;

const SETTINGS = `{
  "limits": {
    "dronesAtOnce": 2,
    "checksAtOnce": 4
  },
  "timeouts": {
    "checkSeconds": 900
  },
  "density": "comfortable"
}
`;

const EDITED = SETTINGS.replace('"comfortable"', '"comfortable",\n  "sounds": false');

/** A file as it was read: the language is the name's extension, and the head carries no mark. */
export const Saved: Story = { args: { text: SETTINGS } };

/**
 * **Save appears only while there is an edit, and carries ⌘S.** A press and the chord do the same thing. A save with
 * nothing changed is not offered, which is the half only a run can hold.
 */
export const Unsaved: Story = {
  args: { text: EDITED, dirty: true },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: /^Save/ }));
    await expect(args.onSave).toHaveBeenCalledTimes(1);
    await userEvent.keyboard("{Meta>}s{/Meta}");
    await expect(args.onSave).toHaveBeenCalledTimes(2);
  },
};

/**
 * A save that found the file changed on disk wrote nothing. **Both are drawn, and neither is touched until a press says
 * which stands**; ⌘S does not save over a moved file.
 */
export const Moved: Story = {
  args: { text: EDITED, dirty: true, moved: { onDisk: SETTINGS.replace("comfortable", "compact") }, onKeepMine: fn(), onTakeDisk: fn() },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText("Changed on disk")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /^Save/ })).toBeNull();
    await userEvent.keyboard("{Meta>}s{/Meta}");
    await expect(args.onSave).not.toHaveBeenCalled();
    await userEvent.click(canvas.getByRole("button", { name: "Keep mine" }));
    await expect(args.onKeepMine).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "Take disk" }));
    await expect(args.onTakeDisk).toHaveBeenCalledTimes(1);
  },
};

/** A picture is shown and not edited, so there is no editor and no unsaved mark. */
export const Picture: Story = {
  args: {
    name: "clock-blocks.png",
    path: "/Users/user/Development/armada/docs/notes/clock-blocks.png",
    type: "image/png",
    imageSrc: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAHgAAABQCAIAAABd+SbeAAAAwUlEQVR42u3QMRGAMBAAwfhJlZKhR0Bk0SACCyiIC9zg4Yvni505A7etjy3cfsxw1/2GW88Z7q/fBho0aNCgQYMGDRo0aNCgQYMGDRo0aNCgQYMGDRp0UWhYOb+gQYMGDRo0aNCgQYMGDRo0aNCgQYMGDRo0aNCgQVeFhpXzCxo0aNCgQYMGDRo0aNCgQYMGDRo0aNCgQYMGDRp0VWhYOb+gQYMGDRo0aNCgQYMGDRo0aNCgQYMGDRo0aNCgQReF/gComBke7NyJcQAAAABJRU5ErkJggg==",
  },
};

/** A read the panel refused: one line, facts only, and the head still names the file. */
export const Refused: Story = {
  args: { name: "recording.mov", path: "/Users/user/Movies/recording.mov", type: "text/plain", refused: "Not a text file" },
};

/** **A markdown file offers Preview and no other file does**: the rendered view has nothing to render of JSON or a picture. */
export const MarkdownOffersPreview: Story = {
  args: { name: "notes.md", path: "/Users/user/Development/armada/docs/notes.md", type: "text/markdown", text: "# Notes\n\nA line.\n" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Preview" })).toHaveAttribute("aria-pressed", "false");
  },
};

export const JsonOffersNoPreview: Story = {
  args: { text: SETTINGS },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Preview" })).toBeNull();
  },
};

/**
 * **Preview opens the rendered file in a floating panel beside it, and follows the edit.** Pressing it again closes it. Typed
 * text is in the preview as it is typed, with nothing to save first.
 */
export const PreviewBesideTheFile: Story = {
  args: { name: "notes.md", path: "/Users/user/Development/armada/docs/notes.md", type: "text/markdown", text: "# Notes\n\nA line.\n" },
  render: (args) => {
    const [text, setText] = useState(args.text ?? "");
    return <FilePanel {...args} text={text} dirty={text !== args.text} onChange={setText} />;
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Preview" }));
    const preview = canvas.getByRole("dialog", { name: "Preview" });
    await expect(within(preview).getByText("Notes")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Preview" })).toHaveAttribute("aria-pressed", "true");
    typeIntoEditor(canvas.getByRole("group", { name: "Editor notes.md" }), "@3  Extra");
    await waitFor(() => expect(within(preview).getByText("A line. Extra")).toBeVisible());
    await userEvent.click(canvas.getByRole("button", { name: "Preview" }));
    await expect(canvas.queryByRole("dialog", { name: "Preview" })).toBeNull();
  },
};
