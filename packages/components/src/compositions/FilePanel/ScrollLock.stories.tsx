import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { lineForTop, topForLine, type Mark } from "./scroll-lock";

/**
 * How a source line and a place in the rendered view stand for each other. The preview's blocks are marks, and a line
 * between two marks is a proportion of the way between their places, so the view moves smoothly through a long block
 * rather than jumping from one to the next. The last mark is the end of the text.
 */
const meta: Meta = { title: "Compositions/File panel/Scroll lock", render: () => <span /> };
export default meta;

type Story = StoryObj;

const MARKS: Mark[] = [
  { line: 1, top: 0 },
  { line: 3, top: 40 },
  { line: 13, top: 240 },
  { line: 21, top: 300 },
];

/** A line that starts a block is that block's place; one inside a block is in proportion; one past the end stays at the end. */
export const LineToPlace: Story = {
  play: async () => {
    await expect(topForLine(MARKS, 1)).toBe(0);
    await expect(topForLine(MARKS, 3)).toBe(40);
    await expect(topForLine(MARKS, 8)).toBe(140);
    await expect(topForLine(MARKS, 21)).toBe(300);
    await expect(topForLine(MARKS, 99)).toBe(300);
    await expect(topForLine(MARKS, 0)).toBe(0);
  },
};

/** The other way, and each is the other's inverse between marks. */
export const PlaceToLine: Story = {
  play: async () => {
    await expect(lineForTop(MARKS, 0)).toBe(1);
    await expect(lineForTop(MARKS, 140)).toBe(8);
    await expect(lineForTop(MARKS, 300)).toBe(21);
    await expect(lineForTop(MARKS, 999)).toBe(21);
    await expect(lineForTop(MARKS, -5)).toBe(1);
    await expect(lineForTop(MARKS, topForLine(MARKS, 17))).toBeCloseTo(17);
  },
};

/** No marks, nothing to follow: both answers are the start. */
export const NoMarks: Story = {
  play: async () => {
    await expect(topForLine([], 5)).toBe(0);
    await expect(lineForTop([], 5)).toBe(1);
  },
};
