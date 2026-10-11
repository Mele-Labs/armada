import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { AutopilotBar, type AutopilotBarProps } from "./AutopilotBar";

const CAPS: AutopilotBarProps["caps"] = [
  { label: "At once", value: "2 of 2 running", fill: 1, reached: true },
  { label: "Spend", value: "$12.40 of $30", fill: 0.41, reached: false },
  { label: "Jobs", value: "4 started · no cap", fill: 0, reached: false },
  { label: "Until", value: "07:00 · 5h 41m left", fill: 0.28, reached: false },
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
};

const meta: Meta<typeof AutopilotBar> = {
  title: "Compositions/Autopilot bar",
  component: AutopilotBar,
  args: BASE,
};
export default meta;

type Story = StoryObj<typeof AutopilotBar>;

/** Running, with a place for every Job it may run at once taken: that cap is lit. */
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

/** Paused: no new picks, and the Jobs it started carry on. */
export const Paused: Story = {
  args: { state: "paused", caps: CAPS.map((cap) => ({ ...cap, reached: false })) },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Paused · started Jobs finish");
    await userEvent.click(canvas.getByRole("button", { name: "Resume" }));
    await expect(args.onResume).toHaveBeenCalledOnce();
    await expect(canvas.queryByRole("button", { name: "Pause" })).toBeNull();
  },
};

/** A cap ended it: the bar says which, and that cap stays lit. */
export const EndedByACap: Story = {
  args: {
    state: "off",
    ended: "Stopped when it reached its spend cap of $30. The Jobs it started finished.",
    caps: [
      { label: "At once", value: "0 of 2 running", fill: 0, reached: false },
      { label: "Spend", value: "$30.20 of $30", fill: 1, reached: true },
      { label: "Jobs", value: "6 started · no cap", fill: 0, reached: false },
      { label: "Until", value: "07:00", fill: 0.9, reached: false },
    ],
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
      reading={read ? { fields: { atOnce: "2", until: "07:00", spend: "30", jobs: "" }, said: { atOnce: true, until: true, spend: true, jobs: false } } : undefined}
    />
  );
}

/**
 * Turned on: the sentence first, and Fleet's reading of it before anything can start. Each setting is
 * editable, says whether the sentence named it, and Start sends what is in the fields.
 */
export const StartForm: Story = {
  args: { state: "off", instruction: undefined },
  render: (args) => <Starting {...args} />,
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.queryByRole("button", { name: "Start Autopilot" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Read it" }));
    await expect(args.onRead).toHaveBeenCalledOnce();
    await expect(canvas.getByLabelText("At once")).toHaveValue("2");
    await expect(canvas.getByText("not said")).toBeInTheDocument();
    await userEvent.type(canvas.getByLabelText("Jobs"), "5");
    await userEvent.click(canvas.getByRole("button", { name: "Start Autopilot" }));
    await expect(args.onStart).toHaveBeenCalledWith({ atOnce: "2", until: "07:00", spend: "30", jobs: "5" });
  },
};

/** Fleet refused a press: said in the bar, where the press was. */
export const Refused: Story = {
  args: { refused: "Not paused: Autopilot is already stopped." },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("alert")).toHaveTextContent("Not paused: Autopilot is already stopped.");
  },
};
