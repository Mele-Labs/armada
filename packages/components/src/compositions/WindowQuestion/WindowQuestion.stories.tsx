import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent } from "storybook/test";

import { WindowQuestion, type WindowQuestionProps } from "./WindowQuestion";

/** The card over a walked page, so each story draws its place in the canvas as the bar draws it over the page. */
const meta: Meta<typeof WindowQuestion> = {
  title: "Compositions/Window question",
  component: WindowQuestion,
  args: {
    from: "Findings page",
    question: "Should the count sit beside the title or under it?",
    options: [
      { label: "Beside", description: "One line, as on the Board" },
      { label: "Under", description: "Leaves the title its whole width" },
    ],
    box: { x: 120, y: 120, width: 220, height: 32 },
    answer: "",
    onAnswer: fn(),
    onChoose: fn(),
    onSend: fn(),
    onFold: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof WindowQuestion>;

/** The place outlined, and the card against it with the question and its options. */
export const Asking: Story = {};

/** Nothing on the page matched: the card still asks, at the top, and says it found no place. */
export const PlaceNotFound: Story = {
  args: { box: null },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The place it asks about is not on this page.")).toBeVisible();
    await expect(canvas.queryByRole("img", { name: "The place asked about" })).toBeNull();
  },
};

/** No options: the question is answered in words alone. */
export const InWordsOnly: Story = {
  args: { options: [] },
};

/** Out to Fleet: no option and no Send takes a second press. */
export const Sending: Story = {
  args: { answer: "and smaller", sending: true },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: /Beside/ })).toBeDisabled();
  },
};

/** Fleet did not take it, and the card says so without losing what was typed. */
export const Refused: Story = {
  args: { answer: "and smaller", refused: "Not sent: Fleet is not running." },
};

/**
 * **An option sends at once; words alone wait for Send.** Send is refused until something is said,
 * and Escape and a press beside the card fold it into the bar, so the page can be used, rather than
 * answering anything.
 */
export const WhatAnswers: Story = {
  render: function Draft(args: WindowQuestionProps) {
    const [answer, setAnswer] = useState("");
    return <WindowQuestion {...args} answer={answer} onAnswer={setAnswer} />;
  },
  play: async ({ args, canvas }) => {
    const send = canvas.getByRole("button", { name: "Send" });
    await expect(send).toBeDisabled();
    await userEvent.type(canvas.getByLabelText("Or in your own words"), "   ");
    await expect(send).toBeDisabled();

    await userEvent.click(canvas.getByRole("button", { name: /Under/ }));
    await expect(args.onChoose).toHaveBeenCalledWith(1);
    await expect(args.onSend).not.toHaveBeenCalled();

    await userEvent.type(canvas.getByLabelText("Or in your own words"), "neither");
    await expect(send).toBeEnabled();
    await userEvent.click(send);
    await expect(args.onSend).toHaveBeenCalled();

    await userEvent.keyboard("{Escape}");
    await expect(args.onFold).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "Look around" }));
    await expect(args.onFold).toHaveBeenCalledTimes(2);
  },
};
