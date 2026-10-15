import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState, type ReactNode } from "react";
import { expect, fn, within } from "storybook/test";
import { AccountMark, AccountPage, AccountProvider, AccountView, SYNC_KINDS, WhatSyncsSheet } from "./Account";
import type { Account, AccountFlow, Profile, Scope, SyncChoice, SyncItem } from "./Account";

/**
 * The account a machine signs in to: the device code, what signing in set up, the profiles and What syncs,
 * and the states that are not the happy path: a sign-in that ended, one that was never set up, a profile
 * signed out, and a shared home that does not exist yet. Each story holds the state a Fleet would, so a
 * press shows its result.
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
  { account_id: 1001, login: "mele-work", active: true, chosen: false },
  { account_id: 1002, login: "nickmele", active: false, chosen: true },
];

/** The acts Fleet would carry, held in state; `spy` hears each press. */
function Held({
  profiles: first = [],
  flow: start = { phase: "idle" },
  syncs: opened = false,
  extra,
  spy,
  children,
}: {
  profiles?: readonly Profile[];
  flow?: AccountFlow;
  syncs?: boolean;
  /** Fields of the account beyond the profiles and the flow: what ended, who left, whether a home exists. */
  extra?: Partial<Account>;
  spy?: Partial<Record<keyof Account, (...args: never[]) => void>>;
  children: (account: Account) => ReactNode;
}) {
  const [profiles, setProfiles] = useState(first);
  const [flow] = useState(start);
  const [open, setOpen] = useState(opened);
  const [kinds, setKinds] = useState(choices);
  const account: Account = {
    ...extra,
    profiles,
    flow,
    syncs: open ? { login: "nickmele", kinds } : null,
    signOut: (login) => {
      spy?.signOut?.(login as never);
      setProfiles((was) => was.filter((one) => one.login !== login));
    },
    openSetting: (setting) => spy?.openSetting?.(setting as never),
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
    <Held profiles={[{ account_id: 1002, login: "nickmele", active: true, chosen: false }]}>{(account) => <AccountView account={account} />}</Held>
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

/** A sign-in that ended without signing in: Fleet's sentence, a mark for how it ended, and the one act that follows. */
const tryAgain = fn();
export const SignInEnded: Story = {
  render: () => (
    <Held flow={{ phase: "idle" }} extra={{ ended: { outcome: "denied", said: "The sign-in was turned down." } }} spy={{ signIn: tryAgain }}>
      {(account) => <AccountView account={account} />}
    </Held>
  ),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("region", { name: "Sign-in ended" })).toHaveTextContent("The sign-in was turned down.");
    await expect(canvas.getByRole("img", { name: "Turned down" })).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Try again" }));
    await expect(tryAgain).toHaveBeenCalledTimes(1);
  },
};

/** Each way a sign-in ends has its own mark, and its tooltip gives the word. */
export const SignInEndedEveryWay: Story = {
  render: () => (
    <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
      <Held extra={{ ended: { outcome: "expired", said: "The code ran out before it was typed." } }}>{(account) => <AccountView account={account} />}</Held>
      <Held extra={{ ended: { outcome: "unreachable", said: "The forge could not be reached: the connection timed out." } }}>
        {(account) => <AccountView account={account} />}
      </Held>
      <Held extra={{ ended: { outcome: "refused", said: "The forge refused the sign-in: the app has the device flow off." } }}>
        {(account) => <AccountView account={account} />}
      </Held>
    </div>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Expired" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Not reached" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Refused" })).toBeVisible();
  },
};

/** Sign-in was refused for a blank setting: Fleet names which, and each blank one has a button into Settings. */
const openSetting = fn();
const openSettings = fn();
export const SignInUnavailable: Story = {
  render: () => (
    <Held
      extra={{ unavailable: { said: "Sign-in is unavailable: the sign-in app and the store address are blank in Settings.", blank: ["account", "store"] } }}
      spy={{ openSetting }}
    >
      {(account) => <AccountView account={account} onSettings={openSettings} />}
    </Held>
  ),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("region", { name: "Sign-in unavailable" })).toHaveTextContent("the sign-in app and the store address are blank");
    await userEvent.click(canvas.getByRole("button", { name: "Store address" }));
    await expect(openSetting).toHaveBeenCalledWith("store");
    await expect(openSettings).toHaveBeenCalledTimes(1);
  },
};

/** Signing a profile out removes it from the list and says who left; the store keeping its row is a plain fact beside it. */
export const SignedOutStoreRemains: Story = {
  render: () => (
    <Held profiles={two}>
      {(account) => (
        <AccountView
          account={{ ...account, left: { login: "mele-work", storeRowRemains: true, said: "The store could not be reached." } }}
        />
      )}
    </Held>
  ),
  play: async ({ canvas }) => {
    await expect(within(canvas.getByRole("region", { name: "Signed out" })).getByRole("listitem", { name: "mele-work" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Store not reached" })).toBeVisible();
    await expect(canvas.getByText("Store still lists this device")).toBeVisible();
  },
};

/** Sign out is on each profile of two or more, and beside Add profile for one. */
const signedOut = fn();
export const SignOutFromProfiles: Story = {
  render: () => (
    <Held profiles={two} spy={{ signOut: signedOut }}>
      {(account) => <AccountView account={account} />}
    </Held>
  ),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Sign out mele-work" }));
    await expect(signedOut).toHaveBeenCalledWith("mele-work");
    await expect(canvas.queryByRole("button", { name: "mele-work" })).toBeNull();
  },
};

/** The shared home is not made yet: the home row is not set up, without the slash a repository would follow it with. */
export const SetUpWithoutHome: Story = {
  render: () => (
    <Held flow={{ phase: "setup", login: "nickmele", repository: "", steps: { identity: "done", store: "done", home: "waiting" } }}>
      {(account) => <AccountView account={account} />}
    </Held>
  ),
  play: async ({ canvas }) => {
    const home = within(canvas.getByRole("listitem", { name: "Shared home" }));
    await expect(home.getByRole("img", { name: "Not set up yet" })).toBeVisible();
    await expect(canvas.getByRole("listitem", { name: "Shared home" })).not.toHaveTextContent("/");
    await expect(home.queryByRole("img", { name: "Setting up" })).toBeNull();
  },
};

/** First take: What syncs stays where it will be, greyed, and its tooltip says why. */
export const WhatSyncsWithoutHomeDisabled: Story = {
  render: () => (
    <Held profiles={two} extra={{ home: false, syncsWithoutHome: "disabled" }}>
      {(account) => <AccountView account={account} />}
    </Held>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "What syncs" })).toBeDisabled();
  },
};

/** Second take: What syncs is not drawn until the home exists. */
export const WhatSyncsWithoutHomeHidden: Story = {
  render: () => (
    <Held profiles={two} extra={{ home: false, syncsWithoutHome: "hidden" }}>
      {(account) => <AccountView account={account} />}
    </Held>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "What syncs" })).toBeNull();
  },
};

/** The page is the account's home: signed out it offers the one act, and a sign-in in flight draws over the profiles. */
export const PageSignedOut: Story = {
  render: () => (
    <Held spy={{ signIn: tryAgain }}>
      {() => <AccountPage />}
    </Held>
  ),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Sign in with GitHub" }));
    await expect(tryAgain).toHaveBeenCalled();
  },
};

/** A second login being added keeps the first in view, with the code over it. */
export const PageAddingAProfile: Story = {
  render: () => (
    <Held profiles={[{ account_id: 1002, login: "nickmele", active: true, chosen: false }]} flow={{ phase: "code", code: "KQXP-4N7D" }}>
      {() => <AccountPage />}
    </Held>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("group", { name: "Device code" })).toHaveTextContent("KQXP-4N7D");
    await expect(canvas.getByRole("button", { name: "Add profile" })).toBeVisible();
  },
};

/** The mark a place that opens the account carries: the turn while a code waits, the end's glyph once it failed. */
export const MarkWhileWaitingAndFailed: Story = {
  render: () => (
    <div style={{ display: "flex", gap: 16 }}>
      <Held flow={{ phase: "code", code: "WDJB-MJHT" }}>{(account) => <AccountMark account={account} />}</Held>
      <Held extra={{ ended: { outcome: "denied", said: "The sign-in was turned down." } }}>{(account) => <AccountMark account={account} />}</Held>
    </div>
  ),
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("img", { name: "Waiting for GitHub" })).toBeVisible();
    await expect(canvas.getByRole("img", { name: "Turned down" })).toBeVisible();
  },
};
