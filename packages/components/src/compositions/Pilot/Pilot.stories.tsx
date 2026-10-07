import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { AttestedMark, PilotAct, PilotButton, PilotConfirm, PilotExits, PilotedBy, type PilotValue } from "./Pilot";

/**
 * Piloting a Job: the act, the confirmation, who has the Job, and the three
 * ways out. The exits are the same drawing on a Session's ledger row and on Job
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
  ask: fn(),
  exit: fn(),
  open: fn(),
  pilotedBy: () => undefined,
  ...over,
});

/** Escalated: Pilot is the primary. Running: the same slot, secondary. */
export const TheAct: Story = {
  render: () => {
    const pilot = value();
    return (
      <PilotAct.Provider value={pilot}>
        <PilotButton jobId="j55" status="escalated" />
        <PilotButton jobId="j53" status="running" />
        <PilotButton jobId="j44" status="awaiting_review" />
      </PilotAct.Provider>
    );
  },
  play: async ({ canvas }) => {
    // A Job at review cannot be piloted, so there are two.
    await expect(canvas.getAllByRole("button", { name: "Pilot" })).toHaveLength(2);
  },
};

/** The confirmation names what each outcome does, and Assist is there and off. */
export const Confirmation: Story = {
  render: () => <PilotConfirm open title="Cap the retry backoff" onConfirm={fn()} onCancel={fn()} />,
  play: async ({ canvasElement }) => {
    const dialog = within(canvasElement.ownerDocument.body);
    await expect(await dialog.findByRole("radio", { name: "Assist" })).toBeDisabled();
    await expect(dialog.getByText("Coming soon.")).toBeInTheDocument();
    await expect(dialog.getByRole("radio", { name: "Take Over" })).toBeChecked();
  },
};

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

/** The Session that has the Job, named. */
export const PilotedByASession: Story = {
  render: () => (
    <PilotAct.Provider value={value({ pilotedBy: () => ({ id: "s9", title: "Cap the retry backoff", number: 55 }) })}>
      <PilotedBy jobId="j55" />
    </PilotAct.Provider>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Cap the retry backoff")).toBeInTheDocument();
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

/** A take over Fleet refused, said in the confirmation, which stays up. */
export const ATakeOverRefused: Story = {
  render: () => <PilotConfirm open title="Cap the retry backoff" said="A person has already taken this Job over." onConfirm={fn()} onCancel={fn()} />,
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement.ownerDocument.body).getByText("A person has already taken this Job over.")).toBeInTheDocument();
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
