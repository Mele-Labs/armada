import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState, type ReactNode } from "react";
import { expect, fn, within } from "storybook/test";
import { AccountProvider, AccountView, SYNC_KINDS, WhatSyncsSheet } from "./Account";
import type { Account, AccountFlow, Profile, Scope, SyncChoice, SyncItem } from "./Account";

/**
 * The account a machine signs in to: the device code, what signing in set up, the profiles and What syncs.
 * Each story holds the state a Fleet would, so a press shows its result.
 */
const meta: Meta = {
  title: "Compositions/Account",
};
export default meta;

type Story = StoryObj;

const SKILLS: readonly SyncItem[] = [
  { name: "humanizer", scope: "global", repo: "" },
  { name: "release-notes", scope: "repo", repo: "armada" },
  { name: "recipe-scaler", scope: "private", repo: "" },
];
const PLUGINS: readonly SyncItem[] = [{ name: "gitnexus", scope: "global", repo: "" }];

const choices = (): SyncChoice[] =>
  SYNC_KINDS.map((kind) => ({
    kind,
    on: false,
    pulling: false,
    items: kind === "skills" ? SKILLS : kind === "plugins" ? PLUGINS : [],
  }));

const two: readonly Profile[] = [
  { login: "mele-work", active: true, chosen: false },
  { login: "nickmele", active: false, chosen: true },
];

/** The acts Fleet would carry, held in state; `spy` hears each press. */
function Held({
  profiles: first = [],
  flow: start = { phase: "idle" },
  syncs: opened = false,
  spy,
  children,
}: {
  profiles?: readonly Profile[];
  flow?: AccountFlow;
  syncs?: boolean;
  spy?: Partial<Record<keyof Account, () => void>>;
  children: (account: Account) => ReactNode;
}) {
  const [profiles, setProfiles] = useState(first);
  const [flow] = useState(start);
  const [open, setOpen] = useState(opened);
  const [kinds, setKinds] = useState(choices);
  const account: Account = {
    profiles,
    flow,
    syncs: open ? { login: "nickmele", kinds } : null,
    signIn: () => spy?.signIn?.(),
    openDevicePage: () => spy?.openDevicePage?.(),
    addProfile: () => spy?.addProfile?.(),
    dismiss: () => undefined,
    switchTo: (login) => {
      spy?.switchTo?.();
      setProfiles((was) => was.map((one) => ({ ...one, active: one.login === login })));
    },
    choose: () => setOpen(true),
    closeSyncs: () => setOpen(false),
    setKind: (kind, on) => setKinds((was) => was.map((one) => (one.kind === kind ? { ...one, on } : one))),
    setScope: (kind, name, scope: Scope) =>
      setKinds((was) =>
        was.map((one) => (one.kind === kind ? { ...one, items: one.items.map((item) => (item.name === name ? { ...item, scope } : item)) } : one)),
      ),
  };
  return (
    <AccountProvider value={account}>
      {children(account)}
      <WhatSyncsSheet />
    </AccountProvider>
  );
}

/** The code over the address it is typed at, and the link that opens that address. */
const openDevicePage = fn();
export const DeviceCode: Story = {
  render: () => (
    <Held flow={{ phase: "code", code: "WDJB-MJHT" }} spy={{ openDevicePage }}>
      {(account) => <AccountView account={account} />}
    </Held>
  ),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("group", { name: "Device code" })).toHaveTextContent("WDJB-MJHT");
    await expect(canvas.getByRole("img", { name: "Waiting for GitHub" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Open github.com/login/device" }));
    await expect(openDevicePage).toHaveBeenCalledTimes(1);
  },
};

/** Signing in sets up the identity, the store and the shared home: a row each, done where it is. */
export const SetUp: Story = {
  render: () => (
    <Held flow={{ phase: "setup", login: "nickmele", repository: "armada-home", steps: { identity: "done", store: "working", home: "waiting" } }}>
      {(account) => <AccountView account={account} />}
    </Held>
  ),
  play: async ({ canvas }) => {
    await expect(within(canvas.getByRole("listitem", { name: "Identity" })).getByRole("img", { name: "Done" })).toBeVisible();
    await expect(within(canvas.getByRole("listitem", { name: "Store" })).getByRole("img", { name: "Setting up" })).toBeVisible();
    await expect(within(canvas.getByRole("listitem", { name: "Shared home" })).queryByRole("img", { name: "Done" })).toBeNull();
  },
};

/** One profile draws no list of profiles, only the act that adds another. */
export const OneProfile: Story = {
  render: () => (
    <Held profiles={[{ login: "nickmele", active: true, chosen: false }]}>{(account) => <AccountView account={account} />}</Held>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Add profile" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: /active$/ })).toBeNull();
  },
};

/** From two profiles up each is a chip with the active one marked, and a profile added on purpose has What syncs. */
export const TwoProfiles: Story = {
  render: () => <Held profiles={two}>{(account) => <AccountView account={account} />}</Held>,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("button", { name: "mele-work, active" })).toHaveAttribute("aria-pressed", "true");
    await expect(canvas.getByRole("button", { name: "nickmele" })).toHaveAttribute("aria-pressed", "false");
    // What syncs is offered on the profile added on purpose, and only there.
    await expect(canvas.getAllByRole("button", { name: "What syncs" })).toHaveLength(1);
    await userEvent.click(canvas.getByRole("button", { name: "nickmele" }));
    await expect(canvas.getByRole("button", { name: "nickmele, active" })).toHaveAttribute("aria-pressed", "true");
    await expect(canvas.getByRole("button", { name: "mele-work" })).toHaveAttribute("aria-pressed", "false");
  },
};

/** Every kind is off until it is turned on; turning one on lists its items, each with a scope. */
export const WhatSyncsOffByDefault: Story = {
  render: () => (
    <Held profiles={two} syncs>
      {() => null}
    </Held>
  ),
  play: async ({ canvas, userEvent }) => {
    const dialog = within(await canvas.findByRole("dialog", { name: "What syncs" }));
    const switches = dialog.getAllByRole("switch");
    for (const one of switches) await expect(one).not.toBeChecked();
    await expect(dialog.getByRole("switch", { name: "Skills" })).toBeVisible();
    await userEvent.click(dialog.getByRole("switch", { name: "Skills" }));
    await expect(dialog.getByRole("switch", { name: "Skills" })).toBeChecked();
    await expect(dialog.getByRole("list", { name: "Skills synced" })).toBeVisible();
    // The kinds not turned on stay off.
    await expect(dialog.getByRole("switch", { name: "Plugins" })).not.toBeChecked();
  },
};

/** A press on the scope mark moves an item to the next scope its kind allows: Plugins have no repository scope. */
export const ScopeMarkCycles: Story = {
  render: () => (
    <Held profiles={two} syncs>
      {() => null}
    </Held>
  ),
  play: async ({ canvas, userEvent }) => {
    const dialog = within(await canvas.findByRole("dialog", { name: "What syncs" }));
    await userEvent.click(dialog.getByRole("switch", { name: "Skills" }));
    await userEvent.click(dialog.getByRole("switch", { name: "Plugins" }));

    // Skills allow all three, in the order global, repository, private, then round.
    await expect(dialog.getByRole("button", { name: "Global, humanizer" })).toBeVisible();
    await userEvent.click(dialog.getByRole("button", { name: "Global, humanizer" }));
    await userEvent.click(dialog.getByRole("button", { name: "Repository, humanizer" }));
    await userEvent.click(dialog.getByRole("button", { name: "Private, humanizer" }));
    await expect(dialog.getByRole("button", { name: "Global, humanizer" })).toBeVisible();

    // Plugins go global, private, global: never to a repository.
    await userEvent.click(dialog.getByRole("button", { name: "Global, gitnexus" }));
    await expect(dialog.getByRole("button", { name: "Private, gitnexus" })).toBeVisible();
    await userEvent.click(dialog.getByRole("button", { name: "Private, gitnexus" }));
    await expect(dialog.getByRole("button", { name: "Global, gitnexus" })).toBeVisible();
    await expect(dialog.queryByRole("button", { name: "Repository, gitnexus" })).toBeNull();
  },
};
