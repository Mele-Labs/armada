import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor } from "storybook/test";

import { SessionRetroRow, elapsedWords, type RetroRowItem, type SessionRetroRowData } from "./SessionRetroRow";

/**
 * A Session's retro in its thread, from the press to the end: one card that stays. Running, it
 * says which stage Fleet is at and for how long; writing, the items settle in a field at a time;
 * written, it folds to one line with a link to the kept retro; failed, Fleet's words.
 */
const meta: Meta<typeof SessionRetroRow> = {
  title: "Compositions/Session retro row",
  component: SessionRetroRow,
  args: { onOpen: fn() },
  decorators: [
    (Story) => (
      <ol style={{ width: "calc(var(--space-12) * 9)", margin: 0, padding: 0, listStyle: "none", background: "var(--bg-raised)" }}>
        <Story />
      </ol>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof SessionRetroRow>;

const SINCE = Date.parse("2026-10-12T14:00:00Z");
const NOW = SINCE + 37_000;

const base: SessionRetroRowData = { id: "retro-1", at: "14:00:00", kind: "retro", stage: "asking_agent", stageSince: SINCE, characters: 0, items: [] };

const ITEMS: RetroRowItem[] = [
  {
    who: "agent",
    landsIn: "armada",
    title: "A reset waited 40 minutes for an answer",
    what: "The agent asked to run git reset --hard and the Job sat for 40 minutes while you were away.",
    fix: "Tell you when an ask has waited ten minutes, and let the agent carry on with other work meanwhile.",
  },
  {
    who: "owner",
    landsIn: "kit",
    title: "A search of the whole disk set off privacy prompts",
    what: "The agent ran find over / for pnpm, and macOS asked you to allow access to Photos, one after another.",
    fix: "Refuse a find that starts at / and tell the agent to search the project folder.",
  },
];

/** Fleet put one question to the live agent and is waiting for its reply. */
export const AskingTheAgent: Story = {
  args: { row: base, now: NOW },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Asking the agent what got in the way");
    await expect(canvas.getByRole("timer")).toHaveTextContent("37s");
  },
};

export const ReadingTheRecord: Story = {
  args: { row: { ...base, stage: "reading_record" }, now: SINCE + 4_000 },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("Reading what Fleet kept of the Job");
  },
};

/** The model has begun: an item is begun and holds room for the fields still to come. */
export const WritingJustBegun: Story = {
  args: { row: { ...base, stage: "writing", characters: 96, items: [{ who: "agent", landsIn: "armada", title: "A reset waited 40 minutes for an answer" }] }, now: NOW },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("96 characters")).toBeInTheDocument();
    await expect(canvas.getByRole("listitem", { name: "A reset waited 40 minutes for an answer" })).toBeInTheDocument();
    await expect(canvas.getByText("Still being written")).toBeInTheDocument();
  },
};

/** One item whole and the next half there. Only the last is marked as still being written. */
export const WritingItemsSettling: Story = {
  args: { row: { ...base, stage: "writing", characters: 812, items: [ITEMS[0]!, { who: "owner", landsIn: "kit", title: "A search of the whole disk set off privacy prompts" }] }, now: NOW },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByText("Still being written")).toHaveLength(1);
    await expect(canvas.getByText("Tell you when an ask has waited ten minutes, and let the agent carry on with other work meanwhile.")).toBeInTheDocument();
  },
};

/** Written: one line and the titles. The link opens the kept retro by its number. */
export const Written: Story = {
  args: { row: { ...base, stage: "written", retro: 3, items: ITEMS }, now: NOW },
  play: async ({ canvas, args }) => {
    await expect(canvas.getByText("Retro 3 written · 2 items")).toBeInTheDocument();
    await expect(canvas.getByText("A reset waited 40 minutes for an answer")).toBeInTheDocument();
    await expect(canvas.queryByRole("timer")).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Open retro" }));
    await expect(args.onOpen).toHaveBeenCalledWith(3);
  },
};

/** Fleet kept the retro and has not told this window what is in it: the count is not guessed. */
export const WrittenItemsNotRead: Story = {
  args: { row: { ...base, stage: "written", retro: 4 }, now: NOW },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("Retro 4 written")).toBeInTheDocument();
  },
};

/** Where the window cannot open a retro, the line stands without the link. */
export const WrittenWithNoWayToOpen: Story = {
  args: { row: { ...base, stage: "written", retro: 3, items: ITEMS }, onOpen: undefined },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Open retro" })).toBeNull();
  },
};

export const Failed: Story = {
  args: { row: { ...base, stage: "failed", why: "The model took longer than 90 seconds and Fleet stopped waiting." } },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("The retro was not written")).toBeInTheDocument();
    await expect(canvas.getByText("The model took longer than 90 seconds and Fleet stopped waiting.")).toBeInTheDocument();
  },
};

export const ElapsedReadsInTheUnitAbove: Story = {
  args: { row: base, now: SINCE + 65_000 },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("timer")).toHaveTextContent(elapsedWords(65_000));
    await expect(canvas.getByRole("timer")).toHaveTextContent("1m 05s");
  },
};

/** Without a fixed clock the time counts on by itself, once a second, while Fleet is still at the stage. */
export const ElapsedKeepsCounting: Story = {
  args: { row: { ...base, stage: "reading_record", stageSince: Date.now() }, now: undefined },
  play: async ({ canvas }) => {
    // `stageSince` is stamped when the file loads, so a slow runner can already read 1s here.
    const first = canvas.getByRole("timer").textContent;
    await waitFor(() => expect(canvas.getByRole("timer").textContent).not.toBe(first), { timeout: 3000 });
  },
};
