import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { RevisionMenu } from "./RevisionMenu";

const meta: Meta<typeof RevisionMenu> = {
  title: "Compositions/Revision menu",
  component: RevisionMenu,
};
export default meta;

type Story = StoryObj<typeof RevisionMenu>;

function Held({ count, start }: { count: number; start: number }) {
  const [revision, setRevision] = useState(start);
  return <RevisionMenu count={count} revision={revision} onPick={setRevision} />;
}

/** It opens on the newest. */
export const OnTheNewest: Story = { render: () => <Held count={3} start={3} /> };

/** An earlier revision picked: the trigger names it, and the menu checks it. */
export const OnAnEarlierOne: Story = { render: () => <Held count={3} start={2} /> };
