import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { SessionList, type SessionRowView } from "./SessionList";

/** Sessions, searchable by PR number, branch, Job id, slot or title. Each row says what matched. */
const meta: Meta<typeof SessionList> = {
  title: "Compositions/Session list",
  component: SessionList,
  args: { query: "", onQuery: fn(), onOpen: fn(), onStart: fn() },
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 8)", padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionList>;

const ROWS: SessionRowView[] = [
  { id: "s7", title: "Flaky store test", state: "failing", said: "Checks failed on #1843", lastTurn: "14:09" },
  { id: "s2", title: "Release notes script", state: "waiting", said: "Waiting on you", lastTurn: "13:48" },
  { id: "s9", state: "blank", said: "Blank: no slot, no branch" },
];

export const AtRest: Story = {
  args: { rows: ROWS },
  play: async ({ canvas, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Flaky store test" }));
    await expect(args.onOpen).toHaveBeenCalledWith("s7");
  },
};

/** A search found it by the pull request, and the row shows that chip. */
export const Found: Story = {
  args: { rows: [{ ...ROWS[0]!, matched: { kind: "pull_request", number: 1843 } }], query: "#1843" },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Matched Pull request #1843" })).toBeInTheDocument();
  },
};
