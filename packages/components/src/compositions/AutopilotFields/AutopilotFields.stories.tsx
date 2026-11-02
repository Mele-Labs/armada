import { useState } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { AutopilotFieldInputs, type AutopilotFields, type AutopilotReading } from "./AutopilotFields";

const FIELDS: AutopilotFields = { atOnce: "2", until: "07:00", spend: "30", sessions: "" };

const meta: Meta<typeof AutopilotFieldInputs> = {
  title: "Compositions/Autopilot fields",
  component: AutopilotFieldInputs,
};
export default meta;

type Story = StoryObj<typeof AutopilotFieldInputs>;

function Editable({ quoted }: { quoted?: AutopilotReading["quoted"] }) {
  const [fields, setFields] = useState(FIELDS);
  return <AutopilotFieldInputs fields={fields} onChange={setFields} {...(quoted === undefined ? {} : { quoted })} />;
}

/** The four settings, each editable. */
export const Plain: Story = {
  render: () => <Editable />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.type(canvas.getByLabelText("Jobs"), "5");
    await expect(canvas.getByLabelText("Jobs")).toHaveValue("5");
  },
};

/** Fleet's reading of a sentence: each setting over the words it came from, and "not said" where the sentence did not say it. */
export const Quoted: Story = {
  render: () => <Editable quoted={{ atOnce: "two at a time", until: "until 7am", spend: "$30 max" }} />,
  play: async ({ canvas }) => {
    await expect(canvas.getByText("“two at a time”")).toBeInTheDocument();
    await expect(canvas.getByText("not said")).toBeInTheDocument();
  },
};
