import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { ProseRefs } from "../../primitives/Prose/Prose";
import type { ProseReference } from "../../primitives/Prose/Prose";
import { QuestionDeck } from "./QuestionDeck";
import { ThreadKeys } from "./thread-keys";

/** What a press on a link hands over. Bridge's opens the ticket in the side panel. */
const open = fn();

const scan = (text: string) =>
  [
    ...text.matchAll(/STORE-\d+/g),
  ].map((one) => ({
    from: one.index,
    to: one.index + one[0].length,
    ref: {
      kind: "link",
      id: `https://store.atlassian.net/browse/${one[0]}`,
      title: one[0],
    } satisfies ProseReference,
  }));

const meta: Meta<typeof QuestionDeck> = {
  title: "Compositions/Question deck",
  component: QuestionDeck,
  args: {
    canSkip: true,
    onAnswer: fn(),
  },
  decorators: [
    (Story) => (
      <ThreadKeys>
        <ProseRefs.Provider
          value={{
            scan,
            open,
          }}
        >
          <Story />
        </ProseRefs.Provider>
      </ThreadKeys>
    ),
  ],
  beforeEach: () => {
    open.mockClear();
  },
};
export default meta;

type Story = StoryObj<typeof QuestionDeck>;

/**
 * A ticket the owner taught Bridge to link links in the question and in an option's description, and a press on it opens it:
 * it does not pick the option the link sits in.
 */
export const ATicketInAQuestionLinks: Story = {
  args: {
    questions: [
      {
        question: "Should this fix STORE-1234 or only report it?",
        header: "Scope",
        multi_select: false,
        options: [
          {
            label: "Fix it",
            description: "Closes STORE-5678 as well",
          },
          {
            label: "Report it",
            description: "Leaves the code alone",
          },
        ],
      },
    ],
  },
  play: async ({ canvas, userEvent }) => {
    await expect(
      canvas.getByRole("link", {
        name: "STORE-1234",
      }),
    ).toBeVisible();
    const inOption = canvas.getByRole("link", {
      name: "STORE-5678",
    });
    await userEvent.click(inOption);
    await expect(open).toHaveBeenCalledTimes(1);
    await expect(open).toHaveBeenCalledWith({
      kind: "link",
      id: "https://store.atlassian.net/browse/STORE-5678",
      title: "STORE-5678",
    });
    await expect(
      canvas.getByRole("radio", {
        name: /Fix it/,
      }),
    ).not.toBeChecked();
    await userEvent.click(
      canvas.getByRole("link", {
        name: "STORE-1234",
      }),
    );
    await expect(open).toHaveBeenCalledTimes(2);
    await expect(
      canvas.getByRole("radio", {
        name: /Fix it/,
      }),
    ).not.toBeChecked();
  },
};

/**
 * What a call asks the person to follow sits under the question and above the choices. The choices keep their keys:
 * a number picks its tile, and Answer sends what was picked.
 */
export const ADetailSitsBetweenTheQuestionAndItsChoices: Story = {
  args: {
    flat: true,
    canSkip: false,
    detail: <p>Update to a build with the fix</p>,
    questions: [
      {
        question: "Did the engineer send the outputs back?",
        header: "",
        multi_select: false,
        options: [
          {
            label: "Sent",
            description: "",
          },
          {
            label: "Not needed",
            description: "",
          },
        ],
      },
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    const detail = canvas.getByText("Update to a build with the fix");
    const sent = canvas.getByRole("radio", {
      name: "Sent",
    });
    await expect(detail).toBeVisible();
    await expect(
      detail.compareDocumentPosition(sent) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).not.toBe(0);
    sent.focus();
    await userEvent.keyboard("2");
    await expect(
      canvas.getByRole("radio", {
        name: "Not needed",
      }),
    ).toBeChecked();
    await userEvent.click(
      canvas.getByRole("button", {
        name: /Answer/,
      }),
    );
    await expect(args.onAnswer).toHaveBeenCalledWith("allow_once", [
      {
        question: "Did the engineer send the outputs back?",
        chosen: [
          "Not needed",
        ],
      },
    ]);
  },
};
