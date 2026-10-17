import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn } from "storybook/test";

import { Prose } from "../../primitives/Prose/Prose";
import { DockQuestions, type DockQuestion } from "./DockQuestions";

/**
 * The questions zone of Helm's dock, drawn at the dock's own width on its sunken ground. A call
 * Helm was held on, with questions from two repositories.
 */
const meta: Meta<typeof DockQuestions> = {
  title: "Compositions/Dock questions",
  component: DockQuestions,
  render: (args) => (
    <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
      <DockQuestions {...args} />
    </div>
  ),
};
export default meta;

type Story = StoryObj<typeof DockQuestions>;

const helmCall: DockQuestion = {
  id: "helm:helm-1",
  repository: "armada",
  label: "Helm needs your permission",
  asked: <span className="mono">gh issue list --milestone Helm</span>,
  detail: "Your settings would have to allow Bash(gh issue list:*).",
  waiting: "18s",
  answers: [
    { id: "allow_once", label: "Allow once", consequence: "Helm runs it now. Nothing is written down." },
    {
      id: "allow_and_remember",
      label: "Allow and remember",
      consequence: "Helm runs it now, and the rule goes into this repository's own settings.",
    },
    { id: "refuse", label: "Refuse", consequence: "Helm is told no, and says what it could not do." },
  ],
  onAnswer: fn(),
  onDiscuss: fn(),
};

const otherCall: DockQuestion = {
  ...helmCall,
  id: "helm:helm-2",
  repository: "shop-01",
  asked: <span className="mono">cargo nextest run -p store</span>,
  detail: "Your settings would have to allow Bash(cargo nextest run:*).",
  waiting: "4m",
  onAnswer: fn(),
  onDiscuss: fn(),
};

/**
 * A call Helm was held on: the card names its repository alone, says which rule would have to
 * allow it, and offers three answers of its own. #1389.
 */
export const HelmNeedsPermission: Story = {
  args: { questions: [helmCall] },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/armada · helm/)).toBeInTheDocument();
    await expect(canvas.queryByText(/job /)).not.toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Allow and remember" })).toBeEnabled();
  },
};

/** What the agent wrote, in markdown: the caller hands it over as `Prose`, and it draws as written. */
export const InMarkdown: Story = {
  name: "A question in markdown",
  args: {
    questions: [
      {
        ...helmCall,
        asked: <Prose text={"Run **exited** checks?\n\n- `drone_count` reads the live set\n- the restart drops it"} />,
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
 * Two repositories. Pressing an answer sends its wire name, and Discuss with Helm names the card
 * it was pressed on.
 */
export const FromTwoRepositories: Story = {
  args: { questions: [helmCall, otherCall] },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getAllByRole("article")).toHaveLength(2);
    await expect(canvas.getByRole("article", { name: /shop-01, helm/ })).toBeVisible();
    await expect(canvas.getByRole("article", { name: /armada, helm/ })).toBeVisible();

    await userEvent.click(canvas.getAllByRole("button", { name: "Refuse" })[1]!);
    await expect(args.questions[1]?.onAnswer).toHaveBeenCalledWith("refuse");
    await expect(args.questions[0]?.onAnswer).not.toHaveBeenCalled();

    await userEvent.click(canvas.getAllByRole("button", { name: "Discuss with Helm" })[1]!);
    await expect(args.questions[1]?.onDiscuss).toHaveBeenCalledTimes(1);
    await expect(args.questions[0]?.onDiscuss).not.toHaveBeenCalled();
  },
};

/** Pressing an answer sends it — the whole of what a card does when Fleet takes it. #936. */
export const CardAnswered: Story = {
  args: { questions: [helmCall] },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Allow once" }));
    await expect(args.questions[0]?.onAnswer).toHaveBeenCalledWith("allow_once");
  },
};

/**
 * Stands in for the dock: a press goes out and `answering` follows on the
 * next render, the way `commands.acting` actually arrives.
 */
function Pressing() {
  const [answering, setAnswering] = useState(false);
  return (
    <DockQuestions
      questions={[
        {
          ...helmCall,
          answering,
          onAnswer: (answer) => {
            helmCall.onAnswer?.(answer);
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
    await userEvent.click(canvas.getByRole("button", { name: "Allow once" }));
    await expect(canvas.getByRole("button", { name: "Allow once" })).toHaveAttribute("aria-busy", "true");
    await expect(canvas.getByRole("button", { name: "Refuse" })).toBeDisabled();
  },
};

/**
 * Reloaded mid-answer: the card mounts already `answering`, with no local
 * press to mark. Every answer is off and none carries the bar — the block
 * has nothing of its own to point at. #1117.
 */
export const CardAnsweringElsewhere: Story = {
  args: { questions: [{ ...helmCall, answering: true }] },
  play: async ({ canvas }) => {
    const only = canvas.getByRole("button", { name: "Allow once" });
    await expect(only).toBeDisabled();
    await expect(only).not.toHaveAttribute("aria-busy", "true");
    await expect(canvas.getByRole("button", { name: "Refuse" })).toBeDisabled();
  },
};

/** A window left open across an answer: the card says so, in Fleet's own words, and stays up. #936. */
export const AlreadyAnswered: Story = {
  args: {
    questions: [
      {
        ...helmCall,
        refusal: "the call being answered is not the one outstanding. Read the calls again and answer the one it names",
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent(/not the one outstanding/);
  },
};

/** Before answering from the dock is wired: the answers are drawn, off, and the card says where to answer. */
export const AnswersNotWiredYet: Story = {
  args: {
    questions: [{ ...helmCall, onAnswer: undefined, onDiscuss: undefined, note: "Open Armada on this machine to answer." }],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Allow once" })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: "Discuss with Helm" })).toBeDisabled();
    await expect(canvas.getByRole("note")).toHaveTextContent("Open Armada on this machine to answer.");
  },
};
