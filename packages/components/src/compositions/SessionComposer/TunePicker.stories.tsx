import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn, userEvent, waitFor } from "storybook/test";

import { TunePicker } from "./TunePicker";

/**
 * The spacious composer's model and effort in one control: the model's name and a meter of the effort on the
 * button, and pressed, the models over an effort slider.
 */
const meta: Meta<typeof TunePicker> = {
  title: "Compositions/Session composer/Tune picker",
  component: TunePicker,
  args: {
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
    said: (model) => (model === null ? "Sonnet" : `${model[0]!.toUpperCase()}${model.slice(1)}`),
    onTune: fn(),
  },
  decorators: [
    (Story) => (
      <div
        style={{
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

type Story = StoryObj<typeof TunePicker>;

const open = async (canvas: Parameters<NonNullable<Story["play"]>>[0]["canvas"]) =>
  userEvent.click(
    canvas.getByRole("button", {
      name: /Sonnet|Opus|Haiku/,
    }),
  );

/** Closed, the button says the model and the effort in use. */
export const OnTheButton: Story = {
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("button", {
        name: /Sonnet/,
      }),
    ).toBeVisible();
    await expect(
      canvas.getByRole("img", {
        name: "Effort: high",
      }),
    ).toBeVisible();
  },
};

/** An effort the Session names none for reads as auto. */
export const EffortAuto: Story = {
  args: {
    effort: null,
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("img", {
        name: "Effort: auto",
      }),
    ).toBeVisible();
  },
};

/** The model the Session follows while it names none is drawn by its own name on the button. */
export const FollowedModel: Story = {
  args: {
    model: null,
    said: () => "Opus 5.5",
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("button", {
        name: /Opus 5\.5/,
      }),
    ).toBeVisible();
  },
};

/** Opened, the model in use has the focus, so the arrows work at once. */
export const OpensOnTheModelInUse: Story = {
  play: async ({ canvas }) => {
    await open(canvas);
    await waitFor(() =>
      expect(
        canvas.getByRole("radio", {
          name: "Sonnet",
        }),
      ).toHaveFocus(),
    );
    await expect(
      canvas.getByRole("radio", {
        name: "Sonnet",
      }),
    ).toBeChecked();
  },
};

/** ↓ and ↑ move between the models, and past either end they wrap to the other. */
export const ArrowsWrap: Story = {
  play: async ({ canvas }) => {
    await open(canvas);
    const radio = (name: string | RegExp) =>
      canvas.getByRole("radio", {
        name,
      });
    await waitFor(() => expect(radio("Sonnet")).toHaveFocus());
    await userEvent.keyboard("{ArrowDown}");
    await expect(radio("Opus")).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}");
    await expect(radio(/^Default/)).toHaveFocus();
    await userEvent.keyboard("{ArrowUp}");
    await expect(radio("Opus")).toHaveFocus();
    await userEvent.keyboard("{ArrowUp}");
    await expect(radio("Sonnet")).toHaveFocus();
  },
};

/** Picking a model keeps the effort in use. */
export const PickKeepsEffort: Story = {
  play: async ({ canvas, args }) => {
    await open(canvas);
    await userEvent.click(
      canvas.getByRole("radio", {
        name: "Opus",
      }),
    );
    await expect(args.onTune).toHaveBeenCalledWith({
      model: "opus",
      effort: "high",
    });
    await userEvent.click(
      canvas.getByRole("radio", {
        name: /^Default/,
      }),
    );
    await expect(args.onTune).toHaveBeenLastCalledWith({
      model: null,
      effort: "high",
    });
  },
};

/** The slider's first step is Auto, which is no effort at all; each step after it is an effort, and the model is kept. */
export const SliderStartsAtAuto: Story = {
  play: async ({ canvas, args }) => {
    await open(canvas);
    const slider = canvas.getByRole("slider", {
      name: "Effort",
    });
    await expect(slider).toHaveAttribute("aria-valuetext", "high");
    await expect(slider).toHaveAttribute("min", "0");
    await fireEvent.change(slider, {
      target: {
        value: "0",
      },
    });
    await expect(args.onTune).toHaveBeenLastCalledWith({
      model: "sonnet",
      effort: null,
    });
    await fireEvent.change(slider, {
      target: {
        value: "1",
      },
    });
    await expect(args.onTune).toHaveBeenLastCalledWith({
      model: "sonnet",
      effort: "low",
    });
  },
};

/** With no effort chosen the slider rests on Auto. */
export const SliderAtAuto: Story = {
  args: {
    effort: null,
  },
  play: async ({ canvas }) => {
    await open(canvas);
    await expect(
      canvas.getByRole("slider", {
        name: "Effort",
      }),
    ).toHaveAttribute("aria-valuetext", "Auto");
  },
};
