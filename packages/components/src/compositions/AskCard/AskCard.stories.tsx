import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";
import { Bot, Compass, Eye, Hand, MessageSquare, OctagonAlert, Scale, ShieldX, Stamp } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { AskActs, AskAnswer, AskAnswers, AskCard, AskChoices, AskPips, AskQuestion } from "./AskCard";

/**
 * An ask as a card, the one frame it is drawn in wherever it is answered: the Cockpit's call, and every
 * ask beside a Job's canvas. A band in the ask's hue naming its kind, over a body that leads with its
 * title and holds the question, its answers as tiles and its acts.
 */
const meta: Meta<typeof AskCard> = {
  title: "Compositions/Ask card",
  component: AskCard,
  args: { icon: Eye, kind: "Review", title: "Ready for review", flow: true, children: null },
  decorators: [
    (Story) => (
      <div style={{ width: "calc(var(--space-12) * 8)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof AskCard>;

/** The band carries the kind, where the ask is and how long it has waited; the title leads the body. */
export const Frame: Story = {
  args: { where: "J-14", at: "2026-10-19T13:54:00Z", now: Date.parse("2026-10-19T14:00:00Z"), children: <AskQuestion quiet>All 3 Checks passed</AskQuestion> },
  play: async ({ canvas }) => {
    const card = canvas.getByRole("region", { name: "Review: Ready for review" });
    await expect(card).toBeVisible();
    await expect(canvas.getByRole("heading", { name: "Ready for review" })).toBeVisible();
    await expect(canvas.getByText("J-14")).toBeVisible();
    await expect(canvas.getByText("6m")).toBeVisible();
  },
};

const KINDS = [
  { icon: Stamp, kind: "Approve dispatch", title: "Waiting for your approval" },
  { icon: Eye, kind: "Review", title: "Ready for review" },
  { icon: Bot, kind: "Drone question", title: "Which clock does the fixture pin?" },
  { icon: Scale, kind: "Judge question", title: "Is the retry cap in scope?" },
  { icon: Hand, kind: "Command held", title: "pnpm add -D reselect" },
  { icon: ShieldX, kind: "Check held", title: "store is held" },
  { icon: MessageSquare, kind: "Plan question", title: "Split the clock out, or wrap it?" },
  { icon: Compass, kind: "Commander asks", title: "The cost cap stopped a retry" },
  { icon: OctagonAlert, kind: "Stopped", title: "This Job stopped" },
] as const;

/** Every kind the app asks in, one frame: each is named by its band and its title. */
export const EveryBandKind: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-3)" }}>
      {KINDS.map((one) => (
        <AskCard key={one.kind} flow icon={one.icon} kind={one.kind} title={one.title}>
          <AskQuestion quiet>{one.title}</AskQuestion>
        </AskCard>
      ))}
    </div>
  ),
  play: async ({ canvas }) => {
    for (const one of KINDS) await expect(canvas.getByRole("region", { name: `${one.kind}: ${one.title}` })).toBeVisible();
  },
};

const pick = fn();

const OPTIONS = [
  { id: "a", label: "A fake the test sets", description: "The fixture owns its clock", recommended: true },
  { id: "b", label: "The real one, stopped", description: "Tests freeze the wall clock" },
  { id: "c", label: "Keep three" },
] as const;

/** Answers carry the number that picks them, a standing one carries its own key, and a press picks. */
export const NumberedAnswers: Story = {
  args: { kind: "Plan question", icon: MessageSquare, title: undefined },
  render: (args) => {
    return (
      <AskCard {...args}>
        <AskQuestion>Which clock does the fixture pin?</AskQuestion>
        <AskChoices label="Which clock does the fixture pin?">
          <AskAnswers>
            {OPTIONS.map((one, index) => (
              <AskAnswer key={one.id} one={one} mark={String(index + 1)} picked={one.id === "b"} onPick={() => pick(one.id)} />
            ))}
          </AskAnswers>
          <AskAnswers standing>
            <AskAnswer one={{ label: "Make the best decision", standing: true }} mark="b" picked={false} onPick={() => pick("best")} />
            <AskAnswer one={{ label: "Just get it done", standing: true }} mark="g" picked={false} onPick={() => pick("done")} />
          </AskAnswers>
        </AskChoices>
        <AskActs>
          <Button variant="primary">Send</Button>
        </AskActs>
      </AskCard>
    );
  },
  play: async ({ canvas }) => {
    const group = canvas.getByRole("radiogroup", { name: "Which clock does the fixture pin?" });
    await expect(group).toBeVisible();
    const radios = canvas.getAllByRole("radio");
    await expect(radios).toHaveLength(5);
    // The number on a tile is what picks it; the standing pair carry their own keys.
    await expect(canvas.getByRole("radio", { name: /^1.*A fake the test sets/ })).toBeVisible();
    await expect(canvas.getByRole("radio", { name: /^2.*The real one, stopped/ })).toHaveAttribute("aria-checked", "true");
    await expect(canvas.getByRole("radio", { name: /^3.*Keep three/ })).toHaveAttribute("aria-checked", "false");
    await expect(canvas.getByRole("radio", { name: /^b.*Make the best decision/ })).toBeVisible();
    await expect(canvas.getByRole("radio", { name: /^g.*Just get it done/ })).toBeVisible();
    // The agent's recommendation and its line under the answer are drawn.
    await expect(canvas.getByText("Recommended")).toBeVisible();
    await expect(canvas.getByText("The fixture owns its clock")).toBeVisible();
    await userEvent.click(canvas.getByRole("radio", { name: /^3.*Keep three/ }));
    await expect(pick).toHaveBeenLastCalledWith("c");
    await userEvent.click(canvas.getByRole("radio", { name: /^g.*Just get it done/ }));
    await expect(pick).toHaveBeenLastCalledWith("done");
  },
};

/** An answer pressed hands the press up; a tile that is the one in flight keeps its look, and the rest cannot be pressed. */
export const WaitingOnFleet: Story = {
  args: { kind: "Plan question", icon: MessageSquare, title: undefined },
  render: (args) => {
    return (
      <AskCard {...args}>
        <AskQuestion>Which clock does the fixture pin?</AskQuestion>
        <AskChoices label="Which clock">
          <AskAnswers>
            {OPTIONS.map((one, index) => (
              <AskAnswer key={one.id} one={one} mark={String(index + 1)} picked={one.id === "a"} pending={one.id === "a"} held pendingSays="Waiting on Fleet" onPick={() => pick(one.id)} />
            ))}
          </AskAnswers>
        </AskChoices>
      </AskCard>
    );
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("radio", { name: /^1.*A fake the test sets/ })).toHaveAttribute("aria-busy", "true");
    await expect(canvas.getByRole("radio", { name: /^1.*A fake the test sets/ })).toBeEnabled();
    await expect(canvas.getByRole("radio", { name: /^2.*The real one, stopped/ })).toBeDisabled();
    await expect(canvas.getByRole("radio", { name: /^3.*Keep three/ })).toBeDisabled();
  },
};

/** Where a question stands among its decisions: a pip each, named by the decision it stands for. */
export const Decisions: Story = {
  args: { kind: "Plan question", icon: MessageSquare, title: undefined },
  render: (args) => (
    <AskCard {...args}>
      <AskQuestion>
        Pin the clock in the fixtures, or add a fake?
        <AskPips
          decisions={[
            { id: "shape", question: "Split it out or wrap it" },
            { id: "tests", question: "Pin the clock or add a fake" },
            { id: "land", question: "Land on its own branch" },
          ]}
          at={1}
        />
      </AskQuestion>
    </AskCard>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("list", { name: "Decisions" })).toBeVisible();
    await expect(canvas.getAllByRole("img", { name: /^(Split|Pin|Land)/ })).toHaveLength(3);
    await expect(canvas.getByRole("img", { name: "Pin the clock or add a fake" })).toHaveAttribute("aria-current", "step");
    await expect(canvas.getByRole("img", { name: "Land on its own branch" })).not.toHaveAttribute("aria-current");
  },
};
