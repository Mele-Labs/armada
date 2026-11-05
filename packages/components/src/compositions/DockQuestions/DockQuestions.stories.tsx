import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn } from "storybook/test";

import { Prose } from "../../primitives/Prose/Prose";
import { DockQuestions, type DockQuestion } from "./DockQuestions";

/**
 * Calls held for a person, drawn on a sunken ground, with questions from two repositories.
 */
const meta: Meta<typeof DockQuestions> = {
  title: "Compositions/Dock questions",
  component: DockQuestions,
  render: (args) => (
    <div
      style={{
        width: "var(--w-dock)",
        background: "var(--bg-sunken)",
        padding: "var(--space-4)",
      }}
    >
      <DockQuestions {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof DockQuestions>;

const heldCall: DockQuestion = {
  id: "call:call-1",
  repository: "armada",
  label: "A job needs your permission",
  asked: <span className="mono">gh issue list --milestone Release</span>,
  detail: "Your settings would have to allow Bash(gh issue list:*).",
  waiting: "18s",
  answers: [
    {
      id: "allow_once",
      label: "Allow once",
      consequence: "Runs it now. Nothing is written down.",
    },
    {
      id: "allow_and_remember",
      label: "Allow and remember",
      consequence: "Runs it now, and the rule goes into this repository's own settings.",
    },
    {
      id: "refuse",
      label: "Refuse",
      consequence: "The job is told no, and says what it could not do.",
    },
  ],
  onAnswer: fn(),
};

const otherCall: DockQuestion = {
  ...heldCall,
  id: "call:call-2",
  repository: "shop-01",
  asked: <span className="mono">cargo nextest run -p store</span>,
  detail: "Your settings would have to allow Bash(cargo nextest run:*).",
  waiting: "4m",
  onAnswer: fn(),
};

/**
 * A held call: the card names its repository alone, says which rule would have to
 * allow it, and offers three answers of its own. #1389.
 */
export const NeedsPermission: Story = {
  args: {
    questions: [
      heldCall,
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("armada")).toBeInTheDocument();
    await expect(canvas.queryByText(/job /)).not.toBeInTheDocument();
    await expect(
      canvas.getByRole("button", {
        name: "Allow and remember",
      }),
    ).toBeEnabled();
  },
};

/** What the agent wrote, in markdown: the caller hands it over as `Prose`, and it draws as written. */
export const InMarkdown: Story = {
  name: "A question in markdown",
  args: {
    questions: [
      {
        ...heldCall,
        asked: (
          <Prose
            text={
              "Run **exited** checks?\n\n- `drone_count` reads the live set\n- the restart drops it"
            }
          />
        ),
        detail: <Prose text={"A customer is **still charged** the full price by `total()`."} />,
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("exited").tagName).toBe("STRONG");
    await expect(canvas.getByText("drone_count").tagName).toBe("CODE");
    await expect(canvas.getByText("the restart drops it").tagName).toBe("LI");
    await expect(canvas.getByText("still charged").tagName).toBe("STRONG");
    await expect(canvas.getByText("total()").tagName).toBe("CODE");
  },
};

/**
 * Two repositories. Pressing an answer sends its wire name from the card it was pressed on.
 */
export const FromTwoRepositories: Story = {
  args: {
    questions: [
      heldCall,
      otherCall,
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getAllByRole("article")).toHaveLength(2);
    await expect(
      canvas.getByRole("article", {
        name: /shop-01/,
      }),
    ).toBeVisible();
    await expect(
      canvas.getByRole("article", {
        name: /armada/,
      }),
    ).toBeVisible();

    await userEvent.click(
      canvas.getAllByRole("button", {
        name: "Refuse",
      })[1]!,
    );
    await expect(args.questions[1]?.onAnswer).toHaveBeenCalledWith("refuse");
    await expect(args.questions[0]?.onAnswer).not.toHaveBeenCalled();
  },
};

/** Pressing an answer sends it — the whole of what a card does when Fleet takes it. #936. */
export const CardAnswered: Story = {
  args: {
    questions: [
      heldCall,
    ],
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Allow once",
      }),
    );
    await expect(args.questions[0]?.onAnswer).toHaveBeenCalledWith("allow_once");
  },
};

/**
 * Stands in for the window: a press goes out and `answering` follows on the
 * next render, the way `commands.acting` actually arrives.
 */
function Pressing() {
  const [answering, setAnswering] = useState(false);
  return (
    <DockQuestions
      questions={[
        {
          ...heldCall,
          answering,
          onAnswer: (answer) => {
            heldCall.onAnswer?.(answer);
            setAnswering(true);
          },
        },
      ]}
    />
  );
}

/**
 * Pressed, and Fleet has not answered. The pressed answer waits and the rest
 * of the group is off — `answering` alone cannot say which one was pressed,
 * so the card remembers it locally. #1117.
 */
export const CardPending: Story = {
  render: () => <Pressing />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(
      canvas.getByRole("button", {
        name: "Allow once",
      }),
    );
    await expect(
      canvas.getByRole("button", {
        name: "Allow once",
      }),
    ).toHaveAttribute("aria-busy", "true");
    await expect(
      canvas.getByRole("button", {
        name: "Refuse",
      }),
    ).toBeDisabled();
  },
};

/**
 * Reloaded mid-answer: the card mounts already `answering`, with no local
 * press to mark. Every answer is off and none carries the bar — the block
 * has nothing of its own to point at. #1117.
 */
export const CardAnsweringElsewhere: Story = {
  args: {
    questions: [
      {
        ...heldCall,
        answering: true,
      },
    ],
  },
  play: async ({ canvas }) => {
    const only = canvas.getByRole("button", {
      name: "Allow once",
    });
    await expect(only).toBeDisabled();
    await expect(only).not.toHaveAttribute("aria-busy", "true");
    await expect(
      canvas.getByRole("button", {
        name: "Refuse",
      }),
    ).toBeDisabled();
  },
};

/** A window left open across an answer: the card says so, in Fleet's own words, and stays up. #936. */
export const AlreadyAnswered: Story = {
  args: {
    questions: [
      {
        ...heldCall,
        refusal:
          "the call being answered is not the one outstanding. Read the calls again and answer the one it names",
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent(/not the one outstanding/);
  },
};

/** Before answering is wired: the answers are drawn, off, and the card says where to answer. */
export const AnswersNotWiredYet: Story = {
  args: {
    questions: [
      {
        ...heldCall,
        onAnswer: undefined,
        note: "Open Armada on this machine to answer.",
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("button", {
        name: "Allow once",
      }),
    ).toBeDisabled();
    await expect(canvas.getByRole("note")).toHaveTextContent(
      "Open Armada on this machine to answer.",
    );
  },
};
