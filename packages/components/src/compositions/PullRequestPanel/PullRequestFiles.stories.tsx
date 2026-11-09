import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, spyOn, within } from "storybook/test";
import type { PullRequestFile } from "@armada/protocol";

import { PullRequestFilesView } from "./PullRequestFiles";

const rows: PullRequestFile[] = Array.from(
  {
    length: 14,
  },
  (_, at) => ({
    path: `crates/example/src/part-${String(at).padStart(2, "0")}.rs`,
    status: "modified",
    additions: 3,
    deletions: 1,
  }),
);

/** The Files tab as Bridge hosts it: a scroller held in a box that clips, so a stray scroll shows as the box moving. */
const meta = {
  title: "Compositions/Pull request files",
  component: PullRequestFilesView,
  args: {
    rows,
    patches: {
      state: "read",
      files: rows.map((row) => ({
        path: row.path,
        diff: {
          files: [],
          emptyNote: "The forge sent no patch for this file.",
        },
      })),
    },
  },
  decorators: [
    (Story) => (
      <div
        data-testid="clip"
        style={{
          height: "24rem",
          overflow: "hidden",
        }}
      >
        <div
          data-testid="scroller"
          style={{
            height: "100%",
            overflowY: "auto",
          }}
        >
          <Story />
          <div
            style={{
              height: "8rem",
            }}
          />
        </div>
      </div>
    ),
  ],
} satisfies Meta<typeof PullRequestFilesView>;

export default meta;
type Story = StoryObj<typeof meta>;

/** A press on the last file scrolls the diff's own scroller and nothing around it: the clipping box above it stays put. */
export const PressingALateFile: Story = {
  play: async ({ canvas, userEvent }) => {
    const scroller = canvas.getByTestId("scroller");
    const into = spyOn(Element.prototype, "scrollIntoView");
    const to = spyOn(Element.prototype, "scrollTo");
    try {
      await userEvent.click(
        within(scroller).getByRole("button", {
          name: (name) => name.includes("part-13.rs"),
        }),
      );
      await expect(into).not.toHaveBeenCalled();
      await expect(to).toHaveBeenCalled();
      await expect(to.mock.contexts[0]).toBe(scroller);
      await expect(canvas.getByTestId("clip").scrollTop).toBe(0);
    } finally {
      into.mockRestore();
      to.mockRestore();
    }
  },
};
