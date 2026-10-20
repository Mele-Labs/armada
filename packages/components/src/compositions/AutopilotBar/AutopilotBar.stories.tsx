import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { AutopilotBar, type AutopilotBarProps } from "./AutopilotBar";

const CAPS: AutopilotBarProps["caps"] = [
  { label: "At once", value: "2 of 2 running", fill: 1, reached: true },
  { label: "Spend", value: "$12.40 of $30", fill: 0.41, reached: false },
  { label: "Sessions", value: "4 started · no cap", fill: 0, reached: false },
  { label: "Until", at: "2026-10-19T07:00:00Z", value: "5h 41m left", fill: 0.28, reached: false },
];

const BASE: AutopilotBarProps = {
  state: "running",
  instruction: "Work through the open issues that don't need design decisions. No more than two at a time, until 7am, $30 max.",
  caps: CAPS,
  stopping: "Stop ends Autopilot. #1850, picked and not started, is withdrawn. #1820 and #1834 keep going to the end.",
  onPause: fn(),
  onResume: fn(),
  onStop: fn(),
  starting: false,
  onStarting: fn(),
  sentence: "",
  onSentence: fn(),
  onRead: fn(),
  onStart: fn(),
  editing: false,
  onEditing: fn(),
  current: { atOnce: "2", until: "07:00", spend: "30", sessions: "" },
  onSave: fn(),
};

/** Fleet's reading of the sentence below, with the words it read each setting from. Sessions it did not say. */
const READING: NonNullable<AutopilotBarProps["reading"]> = {
  fields: { atOnce: "2", until: "07:00", spend: "30", sessions: "" },
  quoted: { atOnce: "two at a time", until: "until 7am", spend: "$30 max" },
};

const meta: Meta<typeof AutopilotBar> = {
  title: "Compositions/Autopilot bar",
  component: AutopilotBar,
  args: BASE,
};
export default meta;

type Story = StoryObj<typeof AutopilotBar>;

/** Running, with a place for every Session it may run at once taken: that cap is lit. */
export const Running: Story = {};

/**
 * Turning the switch off asks first, in the bar, and says what Stop does. Keep running is where focus
 * lands, so Enter changes nothing.
 */
export const TurningItOff: Story = {
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("switch", { name: "Autopilot" }));
    const asked = canvas.getByRole("group", { name: "Stop Autopilot" });
    await expect(asked).toHaveTextContent("#1850, picked and not started, is withdrawn");
    await expect(canvas.getByRole("switch", { name: "Autopilot" })).toBeChecked();
    await userEvent.keyboard("{Enter}");
    await expect(args.onStop).not.toHaveBeenCalled();
    await expect(canvas.queryByRole("group", { name: "Stop Autopilot" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Stop" }));
    await userEvent.click(canvas.getByRole("button", { name: "Stop Autopilot" }));
    await expect(args.onStop).toHaveBeenCalledOnce();
  },
};

/** Paused: no new picks, and the Sessions it started carry on. */
export const Paused: Story = {
  args: { state: "paused", caps: CAPS.map((cap) => ({ ...cap, reached: false })) },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Paused · started Jobs finish");
    await userEvent.click(canvas.getByRole("button", { name: "Resume" }));
    await expect(args.onResume).toHaveBeenCalledOnce();
    await expect(canvas.queryByRole("button", { name: "Pause" })).toBeNull();
  },
};

/** Edit caps, driven as Bridge drives it: open, saved, and closed once Fleet takes it. */
function Capped(props: AutopilotBarProps) {
  const [editing, setEditing] = useState(false);
  return (
    <AutopilotBar
      {...props}
      editing={editing}
      onEditing={setEditing}
      onSave={(fields) => {
        props.onSave(fields);
        setEditing(false);
      }}
    />
  );
}

/**
 * A cap paused it: the state names the cap, that cap stays lit, and Edit caps raises it in the start
 * form's fields. Resume stays pressable, and Fleet's refusal is said in the bar while the cap holds.
 */
export const PausedByACap: Story = {
  args: {
    state: "paused",
    capped: "Job cap reached",
    current: { atOnce: "2", until: "07:00", spend: "30", sessions: "6" },
    caps: [
      { label: "At once", value: "1 of 2 running", fill: 0.5, reached: false },
      { label: "Spend", value: "$18.60 of $30", fill: 0.62, reached: false },
      { label: "Sessions", value: "6 of 6 started", fill: 1, reached: true },
      { label: "Until", at: "2026-10-19T07:00:00Z", value: "2h 10m left", fill: 0.7, reached: false },
    ],
  },
  render: (args) => <Capped {...args} />,
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Paused · Job cap reached");
    await expect(canvas.getByText("6 of 6 started")).toHaveTextContent("6 of 6 started · reached");
    await userEvent.click(canvas.getByRole("button", { name: "Edit caps" }));
    const form = canvas.getByRole("form", { name: "Edit caps" });
    await expect(form).toHaveTextContent("Raise the cap that paused it and save, then resume.");
    await userEvent.clear(canvas.getByLabelText("Jobs"));
    await userEvent.type(canvas.getByLabelText("Jobs"), "10");
    await userEvent.click(canvas.getByRole("button", { name: "Save caps" }));
    await expect(args.onSave).toHaveBeenCalledWith({ atOnce: "2", until: "07:00", spend: "30", sessions: "10" });
    await expect(canvas.queryByRole("form", { name: "Edit caps" })).toBeNull();
  },
};

/** Resume pressed while the cap still holds: Fleet's refusal, said where the press was. */
export const ResumeRefusedWhileCapped: Story = {
  args: {
    ...PausedByACap.args,
    refused: "Autopilot stays paused: the Session cap is reached. Raise it, then resume",
  },
};

/** Its last pass picked nothing, and Fleet says why. */
export const LastPassPickedNothing: Story = {
  args: { fault: "The tracker did not answer in time." },
};

/** Off, with nothing to say: the switch, and nothing else until it is turned on. */
export const Off: Story = {
  args: { state: "off", instruction: undefined },
};

/** The start form, driven as Bridge drives it: a sentence, read by Fleet, checked, then started. */
function Starting(props: AutopilotBarProps) {
  const [starting, setStarting] = useState(true);
  const [sentence, setSentence] = useState("Work through the open issues that don't need design decisions. Two at a time, until 7am, $30 max.");
  const [read, setRead] = useState(false);
  return (
    <AutopilotBar
      {...props}
      starting={starting}
      onStarting={setStarting}
      sentence={sentence}
      onSentence={setSentence}
      onRead={() => {
        props.onRead();
        setRead(true);
      }}
      reading={read ? READING : undefined}
    />
  );
}

/**
 * Turned on: the sentence first, and Fleet's reading of it before anything can start. Each setting is
 * editable, quotes the words it was read from or says the sentence did not say it, and Start sends what
 * is in the fields.
 */
export const StartForm: Story = {
  args: { state: "off", instruction: undefined },
  render: (args) => <Starting {...args} />,
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("button", { name: "Start Autopilot" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Read it" }));
    await expect(args.onRead).toHaveBeenCalledOnce();
    await expect(canvas.getByLabelText("At once")).toHaveValue("2");
    await expect(canvas.getByText("“two at a time”")).toBeInTheDocument();
    await expect(canvas.getByText("not said")).toBeInTheDocument();
    await userEvent.type(canvas.getByLabelText("Jobs"), "5");
    await userEvent.click(canvas.getByRole("button", { name: "Start Autopilot" }));
    await expect(args.onStart).toHaveBeenCalledWith({ atOnce: "2", until: "07:00", spend: "30", sessions: "5" });
  },
};

/** Fleet's reading, as it stands before anything is changed: each setting over the words it came from. */
export const QuotedStartForm: Story = {
  args: {
    state: "off",
    instruction: undefined,
    starting: true,
    sentence: "Work through the open issues that don't need design decisions. No more than two at a time, until 7am, $30 max.",
    reading: READING,
  },
};

/** Fleet refused a press: said in the bar, where the press was. */
export const Refused: Story = {
  args: { refused: "Not paused: Autopilot is already stopped." },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent("Not paused: Autopilot is already stopped.");
  },
};
