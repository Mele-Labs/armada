import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { AttestedMark, PilotAct, PilotExits, type PilotValue } from "./Pilot";

/**
 * A piloted Job: the three ways out. The exits are the same drawing on a Session's ledger row and on Job
 * detail.
 */
const meta: Meta = {
  title: "Compositions/Pilot",
  decorators: [
    (Story) => (
      <div style={{ display: "flex", gap: "var(--space-3)", padding: "var(--space-8)", background: "var(--bg-raised)" }}>
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj;

const value = (over: Partial<PilotValue> = {}): PilotValue => ({
  exit: fn(),
  ...over,
});

/** All three exits, each sending its own. Attest and supersede are drawn as they are on a ledger row. */
export const TheThreeExits: Story = {
  render: () => {
    const pilot = value();
    return (
      <PilotAct.Provider value={pilot}>
        <PilotExits jobId="j55" />
      </PilotAct.Provider>
    );
  },
  play: async ({ canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Attest complete" }));
    await userEvent.click(canvas.getByRole("button", { name: "Close as superseded" }));
    await expect(canvas.getByRole("button", { name: "Submit for verification" })).toBeInTheDocument();
  },
};

/** An exit Fleet refused: said under the exits, which stay so the person can choose another way out. */
export const AnExitRefused: Story = {
  render: () => (
    <PilotAct.Provider value={value({ said: "A step has not advanced. Submit it for verification, or close the Job as superseded." })}>
      <PilotExits jobId="j55" />
    </PilotAct.Provider>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/A step has not advanced/)).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Submit for verification" })).toBeInTheDocument();
  },
};

/** A Job a person closed on their word: its own mark, and the person's words on its tooltip. Never the passed mark. */
export const Attested: Story = {
  render: () => (
    <>
      <AttestedMark />
      <AttestedMark compact note="landed by hand" />
    </>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("img", { name: "Attested, not verified" })).toHaveLength(2);
    await expect(canvas.getByText("Attested")).toBeInTheDocument();
  },
};
