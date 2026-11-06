import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { SpaciousTake } from "../SessionThread/spacious-take";
import { SessionComposer } from "./SessionComposer";

/**
 * A Session's message box in the spacious take, which only the Job view provides: what waits to go is a tray over
 * the words and not chips in the bar, and the model and effort are one control and not two selects. Drawn
 * without the take, the box keeps its chips and its selects.
 */
const meta: Meta<typeof SessionComposer> = {
  title: "Compositions/Session composer/Spacious take",
  component: SessionComposer,
  args: {
    working: false,
    mode: "auto",
    onMode: fn(),
    model: "sonnet",
    effort: "high",
    models: [
      "haiku",
      "sonnet",
      "opus",
    ],
    efforts: [
      "low",
      "medium",
      "high",
    ],
    onTune: fn(),
    commands: [],
    taggable: [],
    tags: [],
    onTags: fn(),
    drawn: [
      {
        id: "k1",
        title: "pin the clock",
      },
    ],
    onDraw: fn(),
    onRemoveDrawn: fn(),
    onSend: fn(),
  },
  decorators: [
    (Story) => (
      <div
        style={{
          width: "calc(var(--space-12) * 10)",
          padding: "var(--space-8)",
          background: "var(--bg-raised)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionComposer>;

const take: NonNullable<Story["decorators"]> = [
  (Story) => (
    <SpaciousTake.Provider value={true}>
      <Story />
    </SpaciousTake.Provider>
  ),
];

const NOTES = new File(
  [
    "notes",
  ],
  "notes.txt",
  {
    type: "text/plain",
  },
);

const attach = async (canvas: Parameters<NonNullable<Story["play"]>>[0]["canvas"]) =>
  userEvent.upload(canvas.getByLabelText("Files to attach", { selector: "input" }), NOTES);

/** What waits is one tray over the words, and nothing of it is left in the bar. */
export const HeldFilesAreATray: Story = {
  decorators: take,
  play: async ({ canvas, canvasElement }) => {
    await attach(canvas);
    const held = await canvas.findAllByRole("group", {
      name: "Attached",
    });
    await expect(held).toHaveLength(1);
    const tray = held[0]!;
    await expect(tray).toHaveClass("armada-tray");
    await expect(within(tray).getByText("notes.txt")).toBeVisible();
    await expect(within(tray).getByText("pin the clock")).toBeVisible();
    const bar = canvasElement.querySelector<HTMLElement>(".armada-session-composer__bar")!;
    await expect(within(bar).queryByText("notes.txt")).toBeNull();
    await expect(within(bar).queryByText("pin the clock")).toBeNull();
  },
};

/** The tray's Remove takes a held file off. */
export const TrayRemovesAFile: Story = {
  decorators: take,
  play: async ({ canvas }) => {
    await attach(canvas);
    await userEvent.click(
      await canvas.findByRole("button", {
        name: "Remove notes.txt",
      }),
    );
    await expect(canvas.queryByText("notes.txt")).toBeNull();
  },
};

/** The two selects give way to one control that says the model and the effort. */
export const OneTuneControl: Story = {
  decorators: take,
  play: async ({ canvas }) => {
    await expect(
      canvas.queryByRole("combobox", {
        name: "Model",
      }),
    ).toBeNull();
    await expect(
      canvas.queryByRole("combobox", {
        name: "Effort",
      }),
    ).toBeNull();
    await expect(
      canvas.getByRole("button", {
        name: /sonnet/,
      }),
    ).toHaveAttribute("aria-haspopup", "dialog");
    await expect(
      canvas.getByRole("img", {
        name: "Effort: high",
      }),
    ).toBeVisible();
  },
};

/** Without the take, what waits is chips in the bar. */
export const ChipsWithoutTheTake: Story = {
  play: async ({ canvas, canvasElement }) => {
    await attach(canvas);
    const held = await canvas.findAllByRole("group", {
      name: "Attached",
    });
    await expect(held).toHaveLength(1);
    await expect(held[0]).not.toHaveClass("armada-tray");
    await expect(within(held[0]!).getByText("notes.txt")).toBeVisible();
    await expect(
      canvasElement
        .querySelector<HTMLElement>(".armada-session-composer__bar")!
        .contains(held[0]!),
    ).toBe(true);
  },
};

/** Without the take, the model and the effort stay two selects. */
export const SelectsWithoutTheTake: Story = {
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("combobox", {
        name: "Model",
      }),
    ).toHaveValue("sonnet");
    await expect(
      canvas.getByRole("combobox", {
        name: "Effort",
      }),
    ).toHaveValue("high");
    await expect(
      canvas.queryByRole("button", {
        name: /sonnet/,
      }),
    ).toBeNull();
  },
};
