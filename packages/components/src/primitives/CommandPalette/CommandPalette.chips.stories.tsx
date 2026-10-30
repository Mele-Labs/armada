import { useState, type ComponentProps } from "react";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, waitFor } from "storybook/test";

import { CommandPalette, type PaletteChip, type PaletteChipOption } from "./CommandPalette";

const meta: Meta<typeof CommandPalette> = {
  title: "Primitives/CommandPalette",
  component: CommandPalette,
};
export default meta;

type Story = StoryObj<typeof CommandPalette>;

const NONE = "none:";
const REPOSITORIES: PaletteChipOption[] = [{ id: "armada", label: "armada", detail: "last used" }, { id: "site", label: "site" }];
/** `Pocket` and `Pocket launch` both: a token naming the second must not stop at the first. `item` is what `+ item` must not become. */
const PROJECTS: PaletteChipOption[] = [
  { id: NONE, label: "No project" },
  { id: "docs", label: "Docs refresh" },
  { id: "pocket", label: "Pocket" },
  { id: "pocket-launch", label: "Pocket launch" },
  { id: "item", label: "item" },
];

/** The two chips a Task row carries, with every choice a spy. */
function chips(repositories: readonly PaletteChipOption[] = REPOSITORIES): PaletteChip[] {
  return [
    { id: "repository", name: "Repository", marker: "@", options: repositories, chosen: "armada", onChoose: fn() },
    { id: "project", name: "Project", marker: "+", options: PROJECTS, chosen: NONE, onChoose: fn() },
  ];
}

/** Holds what its chips are set to, as the host does, so a choice shows on the chip. */
function Held({ dispatch, ...rest }: ComponentProps<typeof CommandPalette>) {
  const [held, setHeld] = useState<Record<string, string>>({});
  const drawn = dispatch!.chips!.map((chip) => ({
    ...chip,
    chosen: held[chip.id] ?? chip.chosen,
    onChoose: (id: string) => {
      chip.onChoose(id);
      setHeld((now) => ({ ...now, [chip.id]: id }));
    },
  }));
  return <CommandPalette {...rest} dispatch={{ ...dispatch!, chips: drawn }} />;
}

const task = (defaultQuery: string, over: Partial<NonNullable<ComponentProps<typeof CommandPalette>["dispatch"]>> = {}) =>
  ({
    open: true,
    sections: [],
    entries: [],
    searched: "",
    defaultQuery,
    onClose: fn(),
    dispatch: { kind: "task", kinds: ["session", "task"], onDispatch: fn(), onKind: fn(), chips: chips(), ...over },
  }) satisfies Story["args"];

const words = (canvas: { getByText: (text: string) => HTMLElement }, text: string) => canvas.getByText(`“${text}”`);

/**
 * **A dispatch row with chips is stacked** (the owner, 30 Oct 2026): the words on the first line and the chips whole
 * under them, so a long project name is not cut short to fit beside the words.
 */
export const ADispatchRowWithChipsIsStacked: Story = {
  args: task("Link the search box from the home page"),
  render: (args) => <Held {...args} />,
  play: async ({ canvas }) => {
    const title = words(canvas, "Link the search box from the home page");
    for (const name of ["Repository armada", "Project No project"]) {
      await expect(canvas.getByRole("button", { name })).toBeVisible();
      await expect(canvas.getByRole("button", { name }).getBoundingClientRect().top).toBeGreaterThanOrEqual(title.getBoundingClientRect().bottom - 1);
    }
  },
};

/** The Job kind carries no chips and the Task kind two, as the one bar's own. */
function Kinds({ dispatch, ...rest }: ComponentProps<typeof CommandPalette>) {
  const [kind, setKind] = useState<"session" | "task">("session");
  const carried = kind === "task" ? dispatch!.chips : [];
  return <CommandPalette {...rest} dispatch={{ ...dispatch!, kind, chips: carried, onKind: (next) => setKind(next as "session" | "task") }} />;
}

/** **A row without chips stays one line**: the Job's row is as short as it was, and the Task's grows by the chips' line. */
export const ARowWithoutChipsStaysOneLine: Story = {
  args: task("Link the search box"),
  render: (args) => <Kinds {...args} />,
  play: async ({ canvas, userEvent }) => {
    const job = canvas.getByRole("option", { name: /^Start a Job with “Link the search box”/ });
    const oneLine = job.getBoundingClientRect().height;
    await expect(canvas.queryAllByRole("button")).toHaveLength(0);
    await userEvent.keyboard("{Tab}");
    const stacked = await canvas.findByRole("option", { name: /^Add a task “Link the search box”/ });
    await expect(stacked.getBoundingClientRect().height).toBeGreaterThan(oneLine);
    await userEvent.keyboard("{Shift>}{Tab}{/Shift}");
    const back = await canvas.findByRole("option", { name: /^Start a Job with “Link the search box”/ });
    await expect(back.getBoundingClientRect().height).toBe(oneLine);
  },
};

/**
 * **⇥ opens the first chip's list; ↵ chooses and the words have the keys again.** The field keeps the words, and typing
 * goes on where it was.
 */
export const TabOpensTheFirstChipsListAndEnterChooses: Story = {
  args: task("Link the search box"),
  render: (args) => <Held {...args} />,
  play: async ({ args, canvas, userEvent }) => {
    const field = canvas.getByRole("combobox");
    await userEvent.keyboard("{Tab}");
    await expect(canvas.getByRole("button", { name: "Repository armada" })).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getByRole("option", { name: /^armada/ })).toHaveTextContent("last used");
    await expect(canvas.getByRole("option", { name: "site" })).toBeVisible();
    await userEvent.keyboard("{ArrowDown}{Enter}");
    await expect(args.dispatch!.chips![0]!.onChoose).toHaveBeenCalledWith("site");
    await expect(args.dispatch!.chips![0]!.onChoose).toHaveBeenCalledTimes(1);
    await expect(canvas.queryByRole("option", { name: "site" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Repository site" })).toHaveAttribute("aria-expanded", "false");
    await expect(field).toHaveFocus();
    await expect(field).toHaveValue("Link the search box");
    await userEvent.keyboard(" now");
    await expect(canvas.getByRole("option", { name: /^Add a task “Link the search box now”/ })).toBeVisible();
    // Nothing was filed on the way.
    await expect(args.dispatch!.onDispatch).not.toHaveBeenCalled();
  },
};

/** **Esc leaves the chip as it was**, closes only its list, and the palette stays up. */
export const EscapeLeavesAChipUnchanged: Story = {
  args: task("Link the search box"),
  render: (args) => <Held {...args} />,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.keyboard("{Tab}{ArrowDown}");
    await expect(canvas.getByRole("option", { name: "site" })).toBeVisible();
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("option", { name: "site" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Repository armada" })).toBeVisible();
    await expect(args.dispatch!.chips![0]!.onChoose).not.toHaveBeenCalled();
    await expect(args.onClose).not.toHaveBeenCalled();
  },
};

/** **⇥ goes on to the next chip and ⇧⇥ back; past the first, ⇧⇥ is on the words again**, without stepping the kind. */
export const ShiftTabGoesBackThroughTheChips: Story = {
  args: task("Link the search box"),
  render: (args) => <Held {...args} />,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.keyboard("{Tab}");
    await expect(canvas.getByRole("option", { name: "site" })).toBeVisible();
    await userEvent.keyboard("{Tab}");
    await expect(canvas.getByRole("option", { name: "Docs refresh" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Project No project" })).toHaveAttribute("aria-expanded", "true");
    await userEvent.keyboard("{Shift>}{Tab}{/Shift}");
    await expect(canvas.getByRole("option", { name: "site" })).toBeVisible();
    await userEvent.keyboard("{Shift>}{Tab}{/Shift}");
    await expect(canvas.queryByRole("option", { name: "site" })).toBeNull();
    await expect(canvas.getByRole("option", { name: /^Add a task/ })).toBeVisible();
    await expect(args.dispatch!.onKind).not.toHaveBeenCalled();
  },
};

/** **In the words, ⇧⇥ steps the kind back** where ⇥ opens a chip: to the one before Task, not round to the one after. */
export const ShiftTabInTheWordsStepsTheKindBack: Story = {
  args: task("Link the search box", { kinds: ["session", "autopilot", "task"] }),
  render: (args) => <Held {...args} />,
  play: async ({ args, userEvent }) => {
    await userEvent.keyboard("{Shift>}{Tab}{/Shift}");
    await expect(args.dispatch!.onKind).toHaveBeenCalledWith("autopilot");
    await expect(args.dispatch!.onKind).toHaveBeenCalledTimes(1);
  },
};

/** **Past the last chip, ⇥ goes on to the next kind.** */
export const TabPastTheLastChipStepsTheKind: Story = {
  args: task("Link the search box"),
  render: (args) => <Held {...args} />,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.keyboard("{Tab}{Tab}");
    await expect(canvas.getByRole("button", { name: "Project No project" })).toHaveAttribute("aria-expanded", "true");
    await userEvent.keyboard("{Tab}");
    await expect(args.dispatch!.onKind).toHaveBeenCalledWith("session");
    await expect(args.dispatch!.onKind).toHaveBeenCalledTimes(1);
    await expect(canvas.queryByRole("option", { name: "Docs refresh" })).toBeNull();
  },
};

/** **A chip with one option is fixed**: drawn, with no list, and ⇥ passes it by. */
export const AOneOptionChipIsFixed: Story = {
  args: task("Link the search box", { chips: chips([{ id: "armada", label: "armada" }]) }),
  render: (args) => <Held {...args} />,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("img", { name: "Repository armada" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /^Repository/ })).toBeNull();
    await userEvent.keyboard("{Tab}");
    await expect(canvas.getByRole("option", { name: "Docs refresh" })).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Project No project" })).toHaveAttribute("aria-expanded", "true");
  },
};

/** What ↵ files: the words without their tokens, each chip's choice by its id. */
const sends = async (onDispatch: unknown, userEvent: { keyboard: (keys: string) => Promise<void> }, filed: string, chosen: Record<string, string>) => {
  await userEvent.keyboard("{Enter}");
  await waitFor(() => expect(onDispatch).toHaveBeenCalledWith(filed, "task", chosen));
  await expect(onDispatch).toHaveBeenCalledTimes(1);
};

/** **The chips' choices and the stripped words reach `onDispatch`**: the chips as they stand where no token names one. */
export const OnDispatchGetsTheChipsAsTheyStand: Story = {
  args: task("Link the search box"),
  render: (args) => <Held {...args} />,
  play: async (context) => sends(context.args.dispatch!.onDispatch, context.userEvent, "Link the search box", { repository: "armada", project: NONE }),
};

/** **A choice made from a chip's list is what ↵ files.** */
export const OnDispatchGetsAChoiceFromAList: Story = {
  args: task("Link the search box"),
  render: (args) => <Held {...args} />,
  play: async (context) => {
    await context.userEvent.keyboard("{Tab}{ArrowDown}{Enter}");
    await sends(context.args.dispatch!.onDispatch, context.userEvent, "Link the search box", { repository: "site", project: NONE });
  },
};

/** **A marker starts a token at the start or after a space.** The tokens leave the words, and name what the chips hold. */
export const TokensAtTheStartOrAfterASpaceSetTheChips: Story = {
  args: task("+Docs refresh fix the order @site"),
  render: (args) => <Held {...args} />,
  play: async (context) => {
    await expect(context.canvas.getByRole("option", { name: /^Add a task “fix the order”/ })).toBeVisible();
    await expect(context.canvas.getByRole("button", { name: "Repository site" })).toBeVisible();
    await expect(context.canvas.getByRole("button", { name: "Project Docs refresh" })).toBeVisible();
    await sends(context.args.dispatch!.onDispatch, context.userEvent, "fix the order", { repository: "site", project: "docs" });
  },
};

/** **A marker inside a word is plain**: an address, a sum. */
export const AMarkerInsideAWordIsPlain: Story = {
  args: task("mail a@site and c+item"),
  render: (args) => <Held {...args} />,
  play: async (context) => {
    await expect(context.canvas.getByRole("option", { name: /^Add a task “mail a@site and c\+item”/ })).toBeVisible();
    await sends(context.args.dispatch!.onDispatch, context.userEvent, "mail a@site and c+item", { repository: "armada", project: NONE });
  },
};

/** **A `+` and a space is no token**, though a project is named `item` just after it. */
export const APlusBeforeASpaceIsNoToken: Story = {
  args: task("buy + item"),
  render: (args) => <Held {...args} />,
  play: async (context) => {
    await expect(context.canvas.getByRole("button", { name: "Project No project" })).toBeVisible();
    await sends(context.args.dispatch!.onDispatch, context.userEvent, "buy + item", { repository: "armada", project: NONE });
  },
};

/** **The longest name wins, spaces and all**: `+Pocket launch` is that project, not `Pocket` and the word `launch`. */
export const TheLongestProjectNameWins: Story = {
  args: task("Ship the store text +Pocket launch"),
  render: (args) => <Held {...args} />,
  play: async (context) => {
    await expect(context.canvas.getByRole("button", { name: "Project Pocket launch" })).toBeVisible();
    await sends(context.args.dispatch!.onDispatch, context.userEvent, "Ship the store text", { repository: "armada", project: "pocket-launch" });
  },
};
