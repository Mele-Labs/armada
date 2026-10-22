import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, waitFor } from "storybook/test";
import { DropdownMenu, type DropdownMenuEntry } from "./DropdownMenu";

const meta: Meta<typeof DropdownMenu> = {
  title: "Primitives/DropdownMenu",
  component: DropdownMenu,
};
export default meta;

type Story = StoryObj<typeof DropdownMenu>;

/**
 * The row menu, in the item order the component sheet draws: what the row
 * could also do, never a repeat of the row's own button, with Kill last and
 * below a rule. The binding sits right-aligned as a kbd and does not change
 * the item height.
 *
 * The trigger is label-only. The sheet draws a three-dot icon button, and no
 * ellipsis glyph is in the icon registry; see the report.
 */
export const RowMenu: Story = {
  args: {
    defaultOpen: true,
    triggerLabel: "More",
    entries: [
      { kind: "item", id: "worktree", label: "Open the worktree" },
      { kind: "item", id: "copy", label: "Copy job ID", shortcut: "⌘C" },
      { kind: "item", id: "board", label: "Send back to the Job Board" },
      { kind: "separator", id: "rule" },
      { kind: "item", id: "kill", label: "Kill job", shortcut: "x", danger: true },
    ],
    onSelect: fn(),
  },
  /**
   * **The trigger, which every story here bypasses.** `defaultOpen` is how the
   * panel gets drawn on a page, and it means nothing on this sheet has ever
   * pressed the button the menu actually opens from — so the one story that
   * can close it and open it again does.
   *
   * Choosing an item closes the menu and reports the id. Both halves matter: a
   * menu left open covers the row it belongs to, and an id that does not
   * arrive is an item that does nothing.
   */
  play: async ({ args, canvas, userEvent }) => {
    // Waited for rather than read once: the menu enters from opacity 0 at
    // --duration-base, and the first frame of that is a panel no one can see.
    await waitFor(() => expect(canvas.getByRole("menu")).toBeVisible());

    // Esc closes an overlay, per the global tier.
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("menu")).toBeNull();

    await userEvent.click(canvas.getByRole("button", { name: "More" }));
    await waitFor(() => expect(canvas.getByRole("menu")).toBeVisible());

    // Focus goes into the menu as it opens, the arrows step and wrap, and Esc
    // hands focus back to the trigger rather than dropping it on the page.
    await expect(canvas.getByRole("menuitem", { name: /Open the worktree/ })).toHaveFocus();
    await userEvent.keyboard("{ArrowUp}");
    await expect(canvas.getByRole("menuitem", { name: /Kill job/ })).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}{ArrowDown}");
    await expect(canvas.getByRole("menuitem", { name: /Copy job ID/ })).toHaveFocus();
    await userEvent.keyboard("{Escape}");
    await expect(canvas.getByRole("button", { name: "More" })).toHaveFocus();

    await userEvent.click(canvas.getByRole("button", { name: "More" }));
    await waitFor(() => expect(canvas.getByRole("menu")).toBeVisible());
    await userEvent.click(canvas.getByRole("menuitem", { name: /Copy job ID/ }));
    await expect(args.onSelect).toHaveBeenCalledWith("copy");
    await expect(canvas.queryByRole("menu")).toBeNull();
  },
};

/**
 * The picked item takes a checkmark in the shortcut's own slot, and
 * `aria-current` names it for anyone not reading the glyph. A reset action
 * ("All repositories") sits above a separator from the specific choices below
 * it, the repository picker's own shape.
 *
 * **The trigger says it opens a menu, to the eye and to assistive tech.** The
 * owner could not tell the picker was one (2026-09-17), so a label trigger
 * draws `chevron-down` right of its label, and `aria-haspopup` says the same
 * thing to anyone not reading the glyph. The glyph is `aria-hidden`, so the
 * accessible name stays the label alone.
 */
export const WithSelectedItem: Story = {
  args: {
    defaultOpen: true,
    triggerLabel: "All repositories",
    entries: [
      { kind: "item", id: "all", label: "All repositories", selected: true },
      { kind: "separator", id: "rule" },
      { kind: "item", id: "armada", label: "armada" },
      { kind: "item", id: "bridge", label: "bridge" },
    ],
  },
  play: async ({ canvas }) => {
    const trigger = canvas.getByRole("button", { name: "All repositories" });
    await expect(trigger).toHaveAttribute("aria-haspopup", "menu");
    await expect(trigger).toHaveAccessibleName("All repositories");
    // The mark trails the label: drawn, hidden from the tree, and to its right.
    const mark = trigger.lastElementChild;
    await expect(mark).toBeInstanceOf(SVGSVGElement);
    await expect(mark).toBeVisible();
    await expect(mark).toHaveAttribute("aria-hidden", "true");
    const label = document.createRange();
    label.selectNodeContents(trigger.firstChild!);
    await expect(mark!.getBoundingClientRect().left).toBeGreaterThanOrEqual(label.getBoundingClientRect().right);

    await expect(canvas.getByRole("menuitem", { name: "All repositories" })).toHaveAttribute(
      "aria-current",
      "true",
    );
    await expect(canvas.getByRole("menuitem", { name: "armada" })).not.toHaveAttribute("aria-current");
  },
};

/**
 * A panel's filters: each item counts what it holds, and the trigger reads the
 * chosen filter and its count. Zero draws nothing, so a filter holding nothing
 * reads as its label alone.
 *
 * **The count is part of the name, with a space before it.** Drawn inline, the
 * item's accessible name ran the two together as `Checks12`.
 */
export const Counted: Story = {
  args: {
    defaultOpen: true,
    align: "start",
    triggerLabel: "All",
    triggerCount: 34,
    entries: [
      { kind: "item", id: "all", label: "All", count: 34, selected: true },
      { kind: "item", id: "checks", label: "Checks", count: 12 },
      { kind: "item", id: "judges", label: "Judges", count: 0 },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "All 34" })).toHaveAttribute("aria-haspopup", "menu");
    await expect(canvas.getByRole("menuitem", { name: "Checks 12" })).toBeInTheDocument();
    await expect(canvas.getByRole("menuitem", { name: "Judges" })).toBeInTheDocument();
  },
};

/**
 * Each item says what it is doing, trailing in mono, and the trigger reads the
 * chosen one the same way: the drones over a Job's Field (the owner, 22 Oct
 * 2026). A finished one is muted and still choosable. In a narrow frame the
 * name is cut short and the detail never is.
 *
 * **Driven from the keyboard.** ↓ on the trigger opens the menu onto its first
 * item, as Enter and Space do, and a pick hands focus back to the trigger.
 */
export const WithDetails: Story = {
  args: {
    align: "start",
    triggerLabel: "Show the change on the Field",
    triggerDetail: "Writing FieldPanel.tsx",
    triggerAriaLabel: "Show the change on the Field: Writing FieldPanel.tsx",
    triggerTooltip: "Drones",
    entries: [
      { kind: "item", id: "newest", label: "Newest writer" },
      { kind: "separator", id: "rule" },
      { kind: "item", id: "job", label: "Show the change on the Field", detail: "Writing FieldPanel.tsx", selected: true },
      { kind: "item", id: "wire", label: "Check the wire types", detail: "Writing wire.ts" },
      { kind: "item", id: "docs", label: "Write the lens into the docs", detail: "Done", muted: true },
    ],
    onSelect: fn(),
  },
  render: (args) => (
    <div style={{ width: "var(--w-boundary-check)" }}>
      <DropdownMenu {...args} />
    </div>
  ),
  play: async ({ args, canvas, userEvent }) => {
    const trigger = canvas.getByRole("button", { name: "Show the change on the Field: Writing FieldPanel.tsx" });
    // The frame is narrower than the trigger's words: it fits, and the detail is drawn whole.
    const frame = trigger.closest(".armada-dropdown-menu")!.parentElement!;
    await expect(trigger.getBoundingClientRect().right).toBeLessThanOrEqual(frame.getBoundingClientRect().right);
    const detail = canvas.getAllByText("Writing FieldPanel.tsx")[0]!;
    await expect(detail.scrollWidth).toBeLessThanOrEqual(detail.clientWidth);

    trigger.focus();
    await userEvent.keyboard("{ArrowDown}");
    await waitFor(() => expect(canvas.getByRole("menu")).toBeVisible());
    await expect(canvas.getByRole("menuitem", { name: "Newest writer" })).toHaveFocus();
    await expect(canvas.getByRole("menuitem", { name: /^Show the change on the Field/ })).toHaveAttribute("aria-current", "true");
    const done = canvas.getByRole("menuitem", { name: /^Write the lens into the docs/ });
    await expect(done).not.toHaveAttribute("aria-disabled");

    await userEvent.keyboard("{End}{Enter}");
    await expect(args.onSelect).toHaveBeenCalledWith("docs");
    await expect(canvas.queryByRole("menu")).toBeNull();
    await expect(trigger).toHaveFocus();
  },
};

/** A hint is a hover tooltip on any item, a plain one as well as a disabled one that says why it waits. */
export const WithHints: Story = {
  args: {
    defaultOpen: true,
    align: "start",
    triggerLabel: "More",
    entries: [
      { kind: "item", id: "new", label: "New Job", hint: "Starts with a short handoff from this one" },
      { kind: "item", id: "wait", label: "Retro", disabled: true, hint: "Writing" },
    ],
  },
  play: async ({ canvas, userEvent }) => {
    const plain = canvas.getByRole("menuitem", { name: /^New Job/ });
    await expect(plain).toHaveAccessibleDescription("Starts with a short handoff from this one");
    await expect(canvas.getByText("Starts with a short handoff from this one")).not.toBeVisible();
    await userEvent.hover(plain);
    await waitFor(() => expect(canvas.getByText("Starts with a short handoff from this one")).toBeVisible(), { timeout: 3000 });
    // A disabled item takes no pointer, so its hint is reached by focus.
    canvas.getByRole("menuitem", { name: /^Retro/ }).focus();
    await waitFor(() => expect(canvas.getByText("Writing")).toBeVisible(), { timeout: 3000 });
  },
};

/** The section label the contract specifies: --text-2xs in --fg-subtle. */
export const WithSectionLabels: Story = {
  args: {
    defaultOpen: true,
    triggerLabel: "More",
    entries: [
      { kind: "label", id: "l-job", label: "This job" },
      { kind: "item", id: "worktree", label: "Open the worktree" },
      { kind: "item", id: "copy", label: "Copy job ID", shortcut: "⌘C" },
      { kind: "separator", id: "rule" },
      { kind: "label", id: "l-fleet", label: "Fleet" },
      { kind: "item", id: "pause", label: "Freeze dispatch" },
      { kind: "item", id: "kill", label: "Kill job", shortcut: "x", danger: true },
    ],
  },
};

/**
 * Off, because what it would send is already out. **The trigger says so by
 * contrast and not by fading**: `--fg-subtle` text on a `--border-subtle` edge
 * with hover suppressed, the rule every control takes — an opacity would muddy
 * whatever sits behind it. The menu does not open.
 */
export const Disabled: Story = {
  args: {
    triggerLabel: "Stop and wait for me",
    disabled: true,
    entries: [
      { kind: "item", id: "refuse_and_hold", label: "Stop and wait for me" },
      { kind: "item", id: "ask_me", label: "Ask me first" },
      { kind: "item", id: "allow_all", label: "Run it" },
    ],
  },
};

const rowActions: DropdownMenuEntry[] = [
  { kind: "item", id: "worktree", label: "Open the worktree" },
  { kind: "item", id: "copy", label: "Copy job ID", shortcut: "⌘C" },
  { kind: "item", id: "board", label: "Send back to the Job Board" },
  { kind: "separator", id: "rule" },
  { kind: "item", id: "kill", label: "Kill job", shortcut: "x", danger: true },
];

/**
 * A frame that puts the trigger against an edge. The panel is anchor-positioned
 * and flips against its containing block, and `contain: layout` on the frame
 * makes the frame that block — so the story shows the same flip a window edge
 * produces, at a size that fits on the page.
 */
function Frame({ edge, children }: { edge: "left" | "right" | "bottom"; children: React.ReactNode }) {
  return (
    <div className={`armada-dropdown-menu-frame armada-dropdown-menu-frame--${edge}`}>
      {children}
    </div>
  );
}

/**
 * The reported failure. The trigger sits on the leading edge, where a
 * right-aligned menu would open off the screen, so the alignment flips and the
 * menu opens towards the space there is.
 */
export const AtTheLeftEdge: Story = {
  render: () => (
    <Frame edge="left">
      <DropdownMenu defaultOpen triggerLabel="More" entries={rowActions} />
    </Frame>
  ),
};

/**
 * The same menu with room on its own side. Nothing flips: the trigger's
 * alignment is the preference and only the edge overrides it.
 */
export const AtTheRightEdge: Story = {
  render: () => (
    <Frame edge="right">
      <DropdownMenu defaultOpen triggerLabel="More" entries={rowActions} />
    </Frame>
  ),
};

/**
 * No room below — the last row of a list. The menu opens above the trigger
 * rather than squashing to fit, because a menu that shortens itself hides
 * items without saying so.
 */
export const WithNoRoomBelow: Story = {
  render: () => (
    <Frame edge="bottom">
      <DropdownMenu defaultOpen triggerLabel="More" entries={rowActions} />
    </Frame>
  ),
};

/**
 * **A menu long enough to search.** The repository picker at twenty repositories: a filter field
 * at the head, focus lands in it, typing narrows the rows and takes a heading with nothing left
 * under it, Enter takes the first row that can be chosen, and a row listed but not choosable is
 * skipped and does nothing when pressed.
 */
export const Searchable: Story = {
  args: {
    defaultOpen: true,
    triggerLabel: "All repositories",
    searchLabel: "Find a repository",
    entries: [
      { kind: "label", id: "recent", label: "Recent" },
      { kind: "item", id: "recent:/code/billing-api", label: "billing-api" },
      { kind: "separator", id: "rule" },
      { kind: "item", id: "/code/armada", label: "armada" },
      { kind: "label", id: "not-set-up", label: "Not set up" },
      { kind: "item", id: "/code/billing-web", label: "billing-web — Set it up first", disabled: true },
      { kind: "item", id: "/code/checkout", label: "checkout" },
    ],
    onSelect: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await waitFor(() => expect(canvas.getByRole("searchbox", { name: "Find a repository" })).toHaveFocus());

    // Narrowing takes the rows that fall out and the heading over a group left empty.
    await userEvent.keyboard("check");
    await expect(canvas.getAllByRole("menuitem")).toHaveLength(1);
    await expect(canvas.getByRole("menuitem", { name: "checkout" })).toBeVisible();
    await expect(canvas.queryByText("Recent")).toBeNull();

    await userEvent.clear(canvas.getByRole("searchbox"));
    await userEvent.keyboard("zzz");
    await expect(canvas.getByRole("status")).toHaveTextContent("Nothing matches “zzz”.");

    // A row that is listed and not choosable stays in the list, says so, and Enter skips it:
    // the only match left is the one that cannot be chosen, so nothing is picked and the menu stays.
    await userEvent.clear(canvas.getByRole("searchbox"));
    await userEvent.keyboard("billing-w{Enter}");
    await expect(canvas.getByRole("menuitem", { name: /billing-web/ })).toHaveAttribute("aria-disabled", "true");
    await expect(args.onSelect).not.toHaveBeenCalled();
    await expect(canvas.getByRole("menu")).toBeVisible();

    // Enter takes the first row that can be chosen, and the menu closes.
    await userEvent.clear(canvas.getByRole("searchbox"));
    await userEvent.keyboard("billing{Enter}");
    await expect(args.onSelect).toHaveBeenCalledTimes(1);
    await expect(args.onSelect).toHaveBeenCalledWith("recent:/code/billing-api");
    await expect(canvas.queryByRole("menu")).toBeNull();
  },
};

const scopes: DropdownMenuEntry[] = [
  { kind: "item", id: "active", label: "Active", selected: true },
  { kind: "item", id: "ended", label: "Ended" },
];

/**
 * **A search that can be emptied in one press.** The clear button sits inside the right of the
 * field and is drawn only while there is something to clear; pressing it empties the field,
 * gives the rows back and leaves the caret in the field to type again.
 */
export const SearchClears: Story = {
  args: {
    defaultOpen: true,
    triggerLabel: "Filter",
    searchLabel: "Search Work",
    entries: [
      { kind: "item", id: "active", label: "Active" },
      { kind: "item", id: "ended", label: "Ended" },
    ],
    onSelect: fn(),
  },
  play: async ({ canvas, userEvent }) => {
    const field = canvas.getByRole("searchbox", { name: "Search Work" });
    await waitFor(() => expect(field).toHaveFocus());
    await expect(canvas.queryByRole("button", { name: "Clear search" })).toBeNull();

    await userEvent.keyboard("end");
    await expect(canvas.getByRole("button", { name: "Clear search" })).toBeVisible();
    await expect(canvas.getAllByRole("menuitem")).toHaveLength(1);

    // The press empties the field and returns the caret to it, and the button goes with the text.
    await userEvent.click(canvas.getByRole("button", { name: "Clear search" }));
    await expect(field).toHaveValue("");
    await expect(field).toHaveFocus();
    await expect(canvas.getAllByRole("menuitem")).toHaveLength(2);
    await expect(canvas.queryByRole("button", { name: "Clear search" })).toBeNull();
  },
};

const held = fn();

function HeldSearch() {
  const [value, setValue] = useState("");
  return (
    <DropdownMenu
      defaultOpen
      triggerLabel="Filter"
      searchLabel="Search Work"
      search={{
        value,
        onChange: (next) => (held(next), setValue(next)),
        onEnter: () => {},
      }}
      entries={scopes}
    />
  );
}

/**
 * **The same button where the caller holds the text**, as the Cockpit's list does: its search
 * narrows the list and not the menu's own rows, and the clear reports the empty text through
 * `onChange` like any other edit.
 */
export const SearchClearsHeldByTheCaller: Story = {
  render: () => <HeldSearch />,
  play: async ({ canvas, userEvent }) => {
    held.mockClear();
    const field = canvas.getByRole("searchbox", { name: "Search Work" });
    await waitFor(() => expect(field).toHaveFocus());
    await expect(canvas.queryByRole("button", { name: "Clear search" })).toBeNull();

    await userEvent.keyboard("17");
    await expect(field).toHaveValue("17");
    await expect(canvas.getByRole("button", { name: "Clear search" })).toBeVisible();
    // The rows are the caller's to narrow: typing leaves both of them.
    await expect(canvas.getAllByRole("menuitem")).toHaveLength(2);

    await userEvent.click(canvas.getByRole("button", { name: "Clear search" }));
    await expect(held).toHaveBeenLastCalledWith("");
    await expect(field).toHaveValue("");
    await expect(field).toHaveFocus();
    await expect(canvas.queryByRole("button", { name: "Clear search" })).toBeNull();
  },
};
