import { createContext, useContext, useEffect, useRef, useSyncExternalStore } from "react";
import type { CSSProperties, ReactNode } from "react";
import {
  Ban,
  Check,
  CircleDashed,
  Clock,
  Database,
  FolderGit2,
  Globe,
  House,
  Laptop,
  Link as LinkGlyph,
  RotateCw,
  Settings,
  Unplug,
  UserCheck,
  X,
  type LucideIcon,
} from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * The account a machine is signed in to, in the title bar: one GitHub sign-in per machine, by the
 * device flow. Bridge shows a short code, the person types it at github.com/login/device once, and
 * Fleet polls and finishes. Signing in sets up the identity, the central store and the shared home
 * with no further input.
 *
 * **A machine joins one or more profiles, each tied to one GitHub login, and profiles never merge.**
 * What a profile syncs is chosen per kind and per machine, every kind off until the person turns it
 * on, and each synced item has one scope: global, repository, or private to this machine.
 *
 * Nothing is drawn where no account is provided.
 */
export type Scope = "global" | "repo" | "private";

export type SyncKindWord =
  | "skills"
  | "plugins"
  | "agent_file"
  | "global_instructions"
  | "sub_agents"
  | "commands"
  | "mcp_servers"
  | "allowlist"
  | "workflows"
  | "memory";

export type SyncItem = { name: string; scope: Scope; repo: string };

/** One kind in What syncs: off until turned on, pulling while it fills in, then its items. */
export type SyncChoice = { kind: SyncKindWord; on: boolean; pulling: boolean; items: readonly SyncItem[] };

export type Profile = {
  /** The forge's numeric id, which names the profile in every act on it. */
  account_id: number;
  login: string;
  active: boolean;
  /** Added on purpose beside the machine's own: what it syncs is the person's choice. */
  chosen: boolean;
};

export type SetUpId = "identity" | "store" | "home";
export type SetUpState = "waiting" | "working" | "done";

export type AccountFlow =
  | { phase: "idle" }
  /** The device code, shown until it is typed at GitHub. */
  | { phase: "code"; code: string }
  /** What signing in set up, row by row. */
  | { phase: "setup"; login: string; repository: string; steps: Readonly<Record<SetUpId, SetUpState>> };

/** How a sign-in came to nothing. A sign-in that signed in has no end to show. */
export type EndedOutcome = "denied" | "expired" | "unreachable" | "refused";

/** The last sign-in that ended without signing in. `said` is Fleet's own sentence, drawn as it comes. */
export type SignInEnd = { outcome: EndedOutcome; said: string };

/** The Settings category a blank setting lives in: the sign-in app, or the store address. */
export type BlankSetting = "account" | "store";

/** Sign-in refused before it began, because a setting it needs is blank. `said` is Fleet's sentence naming which. */
export type SignInUnavailable = { said: string; blank: readonly BlankSetting[] };

/** A profile signed out, held until the panel is put away. `storeRowRemains`: the store could not be asked to forget the device. */
export type SignedOutNote = { login: string; storeRowRemains: boolean; said?: string };

/** What happens to What syncs on a profile while there is no shared home: drawn greyed with its reason, or not drawn. */
export type SyncsWithoutHome = "disabled" | "hidden";

export type AccountRead = {
  /** Empty is signed out. */
  profiles: readonly Profile[];
  flow: AccountFlow;
  /** The profile whose What syncs is open. */
  syncs: { login: string; kinds: readonly SyncChoice[] } | null;
  /** The last sign-in ended without signing in. Absent while one is in flight or none has ended. */
  ended?: SignInEnd;
  /** Starting a sign-in was refused for a blank setting. */
  unavailable?: SignInUnavailable;
  /** A profile signed out. */
  left?: SignedOutNote;
  /** Whether the shared home exists. Absent is that it does. */
  home?: boolean;
  /** Two takes, while the owner chooses. Absent is `disabled`. */
  syncsWithoutHome?: SyncsWithoutHome;
  /** Where the account opens from: a page of its own, or a panel from the left rail. Two takes, while the owner chooses. Absent is `page`. */
  place?: AccountPlace;
};

/** Where the account lives. Both keep a sign-in in flight when they are put away. */
export type AccountPlace = "page" | "rail";

export type AccountActs = {
  /** Start the device flow. */
  signIn: () => void;
  /** The link on the code card: GitHub's page, where the code is typed. */
  openDevicePage: () => void;
  /** Start the device flow for another login. */
  addProfile: () => void;
  /** Put the panel away. */
  dismiss: () => void;
  switchTo: (login: string) => void;
  /** Open What syncs for a profile. */
  choose: (login: string) => void;
  closeSyncs: () => void;
  setKind: (kind: SyncKindWord, on: boolean) => void;
  setScope: (kind: SyncKindWord, name: string, scope: Scope) => void;
  /** Sign a profile out of this machine. */
  signOut: (login: string) => void;
  /** Point Settings at a blank setting; the view that drew the button then opens Settings. */
  openSetting: (setting: BlankSetting) => void;
};

export type Account = AccountRead & AccountActs;

const Held = createContext<Account | undefined>(undefined);

/** `undefined` offers none, and keeps the tree where an account arrives later from remounting. */
export function AccountProvider({ value, children }: { value: Account | undefined; children: ReactNode }) {
  return <Held.Provider value={value}>{children}</Held.Provider>;
}

/** The account this window is signed in to. `undefined` where there is none to offer. */
export function useAccount(): Account | undefined {
  return useContext(Held);
}

const SCOPE = {
  global: { Glyph: Globe, said: "Global" },
  repo: { Glyph: FolderGit2, said: "Repository" },
  private: { Glyph: Laptop, said: "Private" },
} as const;

const ALL_SCOPES: readonly Scope[] = ["global", "repo", "private"];

const KIND: Record<SyncKindWord, { label: string; scopes: readonly Scope[] }> = {
  skills: { label: "Skills", scopes: ALL_SCOPES },
  plugins: { label: "Plugins", scopes: ["global", "private"] },
  agent_file: { label: "Agent file", scopes: ["global", "private"] },
  global_instructions: { label: "Global instructions", scopes: ["global", "private"] },
  sub_agents: { label: "Sub agents", scopes: ALL_SCOPES },
  commands: { label: "Commands", scopes: ALL_SCOPES },
  mcp_servers: { label: "MCP servers", scopes: ALL_SCOPES },
  allowlist: { label: "Allowlist", scopes: ALL_SCOPES },
  workflows: { label: "Workflows", scopes: ALL_SCOPES },
  memory: { label: "Memory", scopes: ALL_SCOPES },
};

/** The kinds in the order What syncs lists them. */
export const SYNC_KINDS: readonly SyncKindWord[] = [
  "skills",
  "plugins",
  "agent_file",
  "global_instructions",
  "sub_agents",
  "commands",
  "mcp_servers",
  "allowlist",
  "workflows",
  "memory",
];

const DEVICE_PAGE = "github.com/login/device";

/** The mark on a waiting or in-progress row: the turn is the only motion. */
function Turning({ said }: { said: string }) {
  return (
    <Tooltip beside label={said}>
      <span className="armada-account__turn" role="img" aria-label={said}>
        <RotateCw size={16} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/**
 * The account's own content: the device code while signing in, what it set up, with what ended, what was
 * refused and who signed out above the profiles. Absent where there is nothing to show. A sign-in in
 * flight draws over the profiles and never in place of them, so a second login can be added with the
 * first in view. `onSettings` opens Settings for the buttons that name a blank setting; without it they
 * are not drawn.
 */
export function AccountView({ account, onSettings }: { account: Account; onSettings?: () => void }) {
  const here = account.profiles.find((one) => one.active) ?? account.profiles[0];
  const { flow, left, ended, unavailable } = account;
  if (flow.phase === "idle" && here === undefined && left === undefined && ended === undefined && unavailable === undefined) return null;
  return (
    <>
      {flow.phase === "code" ? <DeviceCode code={flow.code} account={account} /> : null}
      {flow.phase === "setup" ? <SetUp flow={flow} /> : null}
      {left === undefined ? null : <SignedOutCard note={left} account={account} />}
      {ended === undefined ? null : <EndedCard ended={ended} account={account} />}
      {unavailable === undefined ? null : <UnavailableCard unavailable={unavailable} account={account} onSettings={onSettings} />}
      {here === undefined ? null : <Profiles account={account} />}
    </>
  );
}

/**
 * What a place that opens the account has to say: a sign-in waiting on the person, or one that has just
 * failed. **Nothing for a signed-out machine or a settled one.** Each place draws the glyph, so the way
 * back to a sign-in that is not in front of them is a mark they can find.
 */
export type Need = { Glyph: LucideIcon; said: string; turning: boolean };

export function accountNeed(account: Account): Need | undefined {
  const { flow, ended, unavailable } = account;
  if (flow.phase === "code") return { Glyph: RotateCw, said: "Waiting for GitHub", turning: true };
  if (flow.phase === "setup" && Object.values(flow.steps).some((one) => one !== "done")) {
    return { Glyph: RotateCw, said: "Setting up", turning: true };
  }
  if (ended !== undefined) return { Glyph: ENDED[ended.outcome].Glyph, said: ENDED[ended.outcome].said, turning: false };
  if (unavailable !== undefined) return { Glyph: CircleDashed, said: "Not set up", turning: false };
  return undefined;
}

/** The mark on a row that opens the account, with its tooltip. Nothing where there is nothing to say. */
export function AccountMark({ account }: { account: Account }) {
  const need = accountNeed(account);
  if (need === undefined) return null;
  if (need.turning) return <Turning said={need.said} />;
  return (
    <Tooltip beside label={need.said}>
      <span className="armada-account__need" role="img" aria-label={need.said}>
        <need.Glyph size={16} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** The same mark at 12px on a button, with no tooltip of its own: the button's title says it. */
export function AccountBadge({ account }: { account: Account }) {
  const need = accountNeed(account);
  if (need === undefined) return null;
  return (
    <span className="armada-account__badge" data-turning={need.turning ? "" : undefined} aria-hidden="true">
      <need.Glyph size={12} strokeWidth={2} />
    </span>
  );
}

// The panel from the rail is open or put away; it is the window's, so anything that opens the account can say so.
let panelOpen = false;
const panelListeners = new Set<() => void>();
const setPanel = (open: boolean) => {
  if (panelOpen === open) return;
  panelOpen = open;
  panelListeners.forEach((on) => on());
};

/** Whether the account panel is open, and the act that opens or puts it away. */
export function useAccountPanel(): [boolean, (open: boolean) => void] {
  const open = useSyncExternalStore(
    (on) => {
      panelListeners.add(on);
      return () => panelListeners.delete(on);
    },
    () => panelOpen,
  );
  return [open, setPanel];
}

/** Opens the account where this window keeps it: the panel, or the page `goToPage` reaches. */
export function showAccount(account: Account | undefined, goToPage: () => void): void {
  if (account?.place === "rail") setPanel(true);
  else goToPage();
}

/** What the page and the panel both hold: the account's states, and the one act signed out has. */
function AccountBody({ account, onSettings }: { account: Account; onSettings?: () => void }) {
  const out = account.profiles.length === 0 && account.flow.phase === "idle" && account.ended === undefined && account.unavailable === undefined;
  return (
    <>
      <AccountView account={account} {...(onSettings === undefined ? {} : { onSettings })} />
      {!out ? null : (
        <section className="armada-account__card" aria-label="Sign in">
          <div className="armada-account__foot" data-alone="">
            <Button variant="primary" size="sm" onClick={account.signIn}>
              Sign in with GitHub
            </Button>
          </div>
        </section>
      )}
    </>
  );
}

/** The account as a page in the work area: the way Settings opens. Nothing where no account is provided. */
export function AccountPage({ onSettings }: { onSettings?: () => void }) {
  const account = useAccount();
  if (account === undefined) return null;
  return (
    <div className="armada-account-page" role="region" aria-label="Account">
      <AccountBody account={account} {...(onSettings === undefined ? {} : { onSettings })} />
    </div>
  );
}

/** The account as a panel from the rail, over the work. A sign-in in flight stays when it is put away. */
export function AccountPanel({ onSettings }: { onSettings?: () => void }) {
  const account = useAccount();
  const [open, setOpen] = useAccountPanel();
  const choosing = account?.syncs != null;
  const closeSyncs = useRef(account?.closeSyncs);
  closeSyncs.current = account?.closeSyncs;
  // The panel and What syncs share the trailing edge: one opening puts the other away.
  useEffect(() => {
    if (choosing) setOpen(false);
  }, [choosing, setOpen]);
  useEffect(() => {
    if (open) closeSyncs.current?.();
  }, [open]);
  if (account === undefined || account.place !== "rail" || !open) return null;
  return (
    <Sheet
      kind="account"
      open
      floating
      through
      title="Account"
      closeLabel="Close panel"
      closeBinding="Esc"
      onClose={() => setOpen(false)}
    >
      <div className="armada-account-panel">
        <AccountBody
          account={account}
          onSettings={() => {
            setOpen(false);
            onSettings?.();
          }}
        />
      </div>
    </Sheet>
  );
}

/** The rail's account row: the login or Sign in, and the mark; at the 48px rail the glyph alone. */
export function AccountRailRow({ account, collapsed, onOpen }: { account: Account; collapsed: boolean; onOpen: () => void }) {
  const login = accountLogin(account);
  const said = login ?? "Sign in";
  const [open] = useAccountPanel();
  const mark = <AccountMark account={account} />;
  const button = (
    <button
      type="button"
      className="armada-account__rail"
      aria-label={login === undefined ? "Sign in" : `Account ${login}`}
      aria-expanded={open}
      onClick={onOpen}
    >
      <UserCheck size={16} strokeWidth={2} aria-hidden="true" />
      {collapsed ? null : <span className="armada-account__rail-name">{said}</span>}
      {collapsed ? <AccountBadge account={account} /> : mark}
    </button>
  );
  return collapsed ? <Tooltip label={said}>{button}</Tooltip> : button;
}

/** The signed-in login the menu's account entry names. Undefined while signed out. */
export function accountLogin(account: Account): string | undefined {
  return (account.profiles.find((one) => one.active) ?? account.profiles[0])?.login;
}

/** The code, large, over the address it is typed at; the turn says Fleet is waiting. */
function DeviceCode({ code, account }: { code: string; account: Account }) {
  return (
    <section className="armada-account__card" aria-label="Device code">
      <header className="armada-account__band">
        <button type="button" className="armada-account__link" aria-label={`Open ${DEVICE_PAGE}`} onClick={account.openDevicePage}>
          <LinkGlyph size={16} strokeWidth={2} aria-hidden="true" />
          <span>{DEVICE_PAGE}</span>
        </button>
        <Turning said="Waiting for GitHub" />
      </header>
      <div className="armada-account__code" role="group" aria-label="Device code">
        {code.split("").map((character, at) =>
          character === "-" ? (
            <span key={at} className="armada-account__code-gap">
              -
            </span>
          ) : (
            <span key={at} className="armada-account__code-cell">
              {character}
            </span>
          ),
        )}
      </div>
    </section>
  );
}

const SET_UP: readonly { id: SetUpId; label: string; Glyph: typeof House }[] = [
  { id: "identity", label: "Identity", Glyph: UserCheck },
  { id: "store", label: "Store", Glyph: Database },
  { id: "home", label: "Shared home", Glyph: House },
];

/** What signing in set up: one row each, a registry glyph and a mark for how far it has got. */
function SetUp({ flow }: { flow: Extract<AccountFlow, { phase: "setup" }> }) {
  return (
    <section className="armada-account__card" aria-label="Set up">
      <ul className="armada-account__rows">
        {SET_UP.map(({ id, label, Glyph }) => {
          const state = flow.steps[id];
          // The home Fleet has named no repository for does not exist yet: that is not a step under way.
          const unset = id === "home" && flow.repository === "" && state !== "done";
          return (
            <li key={id} className="armada-account__row" aria-label={label} data-state={unset ? "waiting" : state}>
              <Tooltip beside label={label}>
                <span className="armada-account__glyph" role="img" aria-label={label}>
                  <Glyph size={16} strokeWidth={2} aria-hidden="true" />
                </span>
              </Tooltip>
              <span className="armada-account__name">
                {id === "store" ? "Store" : flow.login}
                {id === "home" && !unset ? <span className="armada-account__repo">/{flow.repository}</span> : null}
              </span>
              <span className="armada-account__state">
                {unset ? (
                  <Tooltip beside label="Not set up yet">
                    <span className="armada-account__unset" role="img" aria-label="Not set up yet">
                      <CircleDashed size={16} strokeWidth={2} aria-hidden="true" />
                    </span>
                  </Tooltip>
                ) : state === "working" ? (
                  <Turning said="Setting up" />
                ) : state === "done" ? (
                  <Tooltip beside label="Done">
                    <span className="armada-account__done" role="img" aria-label="Done">
                      <Check size={16} strokeWidth={2} aria-hidden="true" />
                    </span>
                  </Tooltip>
                ) : null}
              </span>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/** One glyph per way a sign-in ended, and the word its tooltip gives it. */
const ENDED = {
  denied: { Glyph: Ban, said: "Turned down" },
  expired: { Glyph: Clock, said: "Expired" },
  unreachable: { Glyph: Unplug, said: "Not reached" },
  refused: { Glyph: X, said: "Refused" },
} as const;

/** The setting each blank category names, as Settings calls it. */
const SETTING: Record<BlankSetting, string> = { account: "Sign-in app", store: "Store address" };

/** Puts a notice away: what ended, what was refused and who left stay until it is pressed, or until the next sign-in. */
function Dismiss({ account }: { account: Account }) {
  return (
    <Tooltip beside label="Dismiss">
      <Button variant="ghost" size="sm" iconOnly aria-label="Dismiss" onClick={account.dismiss}>
        <X size={16} strokeWidth={2} aria-hidden="true" />
      </Button>
    </Tooltip>
  );
}

/** A sign-in that ended without signing in: Fleet's sentence beside the mark for how, and the one act that follows. */
function EndedCard({ ended, account }: { ended: SignInEnd; account: Account }) {
  const { Glyph, said } = ENDED[ended.outcome];
  return (
    <section className="armada-account__card" aria-label="Sign-in ended">
      <div className="armada-account__row armada-account__row--said" data-state="ended">
        <Tooltip beside label={said}>
          <span className="armada-account__glyph" role="img" aria-label={said}>
            <Glyph size={16} strokeWidth={2} aria-hidden="true" />
          </span>
        </Tooltip>
        <span className="armada-account__said">{ended.said}</span>
        <Dismiss account={account} />
      </div>
      <div className="armada-account__foot">
        <Button variant="secondary" size="sm" ground="sunken" onClick={account.signIn}>
          Try again
        </Button>
      </div>
    </section>
  );
}

/** Sign-in refused for a blank setting: Fleet's sentence, and a button to each blank setting in Settings. */
function UnavailableCard({
  unavailable,
  account,
  onSettings,
}: {
  unavailable: SignInUnavailable;
  account: Account;
  onSettings: (() => void) | undefined;
}) {
  return (
    <section className="armada-account__card" aria-label="Sign-in unavailable">
      <div className="armada-account__row armada-account__row--said" data-state="ended">
        <Tooltip beside label="Not set up">
          <span className="armada-account__glyph" role="img" aria-label="Not set up">
            <CircleDashed size={16} strokeWidth={2} aria-hidden="true" />
          </span>
        </Tooltip>
        <span className="armada-account__said">{unavailable.said}</span>
        <Dismiss account={account} />
      </div>
      {onSettings === undefined ? null : (
        <div className="armada-account__foot">
          {unavailable.blank.map((setting) => (
            <Button
              key={setting}
              variant="secondary"
              size="sm"
              ground="sunken"
              onClick={() => {
                account.openSetting(setting);
                onSettings();
              }}
            >
              <Settings size={16} strokeWidth={2} aria-hidden="true" />
              {SETTING[setting]}
            </Button>
          ))}
        </div>
      )}
    </section>
  );
}

/** Who signed out, as the set-up rows read in reverse; the store's row is named only where it stays. */
function SignedOutCard({ note, account }: { note: SignedOutNote; account: Account }) {
  return (
    <section className="armada-account__card" aria-label="Signed out">
      <ul className="armada-account__rows">
        <li className="armada-account__row" aria-label={note.login} data-state="gone">
          <Tooltip beside label="Signed out">
            <span className="armada-account__glyph" role="img" aria-label="Signed out">
              <UserCheck size={16} strokeWidth={2} aria-hidden="true" />
            </span>
          </Tooltip>
          <span className="armada-account__name">{note.login}</span>
          <Dismiss account={account} />
        </li>
        {!note.storeRowRemains ? null : (
          <li className="armada-account__row" aria-label="Store" data-state="ended">
            <Tooltip beside label={note.said ?? "Store not reached"}>
              <span className="armada-account__glyph" role="img" aria-label="Store not reached">
                <Unplug size={16} strokeWidth={2} aria-hidden="true" />
              </span>
            </Tooltip>
            <span className="armada-account__said">Store still lists this device</span>
          </li>
        )}
      </ul>
    </section>
  );
}

/** The profiles this machine has joined, one chip each, which is active marked, and the act to add one. */
function Profiles({ account }: { account: Account }) {
  const alone = account.profiles.length < 2;
  const noHome = account.home === false;
  const here = account.profiles.find((one) => one.active) ?? account.profiles[0];
  return (
    <section className="armada-account__card" aria-label="Profiles">
      {/* One profile is the account chip alone; the chips draw once a second joins. */}
      {alone ? null : (
      <ul className="armada-account__rows">
        {account.profiles.map((one) => (
          <li key={one.login} className="armada-account__profile" data-active={one.active ? "" : undefined}>
            <button
              type="button"
              className="armada-account__chip armada-account__chip--wide"
              aria-pressed={one.active}
              aria-label={one.active ? `${one.login}, active` : one.login}
              onClick={() => account.switchTo(one.login)}
            >
              <UserCheck size={12} strokeWidth={2} aria-hidden="true" />
              <span>{one.login}</span>
              {one.active ? (
                <Tooltip beside label="Active">
                  <span className="armada-account__done" role="img" aria-label="Active">
                    <Check size={16} strokeWidth={2} aria-hidden="true" />
                  </span>
                </Tooltip>
              ) : null}
            </button>
            {!one.chosen ? null : !noHome ? (
              <Button variant="ghost" size="sm" onClick={() => account.choose(one.login)}>
                What syncs
              </Button>
            ) : account.syncsWithoutHome === "hidden" ? null : (
              <Tooltip beside label="No shared home yet">
                <Button variant="ghost" size="sm" disabled>
                  What syncs
                </Button>
              </Tooltip>
            )}
            <Button variant="ghost" size="sm" aria-label={`Sign out ${one.login}`} onClick={() => account.signOut(one.login)}>
              Sign out
            </Button>
          </li>
        ))}
      </ul>
      )}
      <div className="armada-account__foot" data-alone={alone ? "" : undefined}>
        <Button variant="secondary" size="sm" ground="sunken" onClick={account.addProfile}>
          Add profile
        </Button>
        {!alone || here === undefined ? null : (
          <Button variant="ghost" size="sm" aria-label={`Sign out ${here.login}`} onClick={() => account.signOut(here.login)}>
            Sign out
          </Button>
        )}
      </div>
    </section>
  );
}

/** The next scope in the order a kind offers, wrapping. */
function nextScope(kind: SyncKindWord, now: Scope): Scope {
  const scopes = KIND[kind].scopes;
  return scopes[(scopes.indexOf(now) + 1) % scopes.length] ?? now;
}

/**
 * What a profile syncs on this machine: one row per kind with a switch, every one off until turned
 * on. A kind turned on pulls, then fills in with its items, each carrying a scope mark that a press
 * moves to the next scope.
 */
export function WhatSyncsSheet() {
  const account = useAccount();
  if (account === undefined || account.syncs === null) return null;
  const { login, kinds } = account.syncs;
  return (
    <Sheet
      kind="what-syncs"
      open
      floating
      title="What syncs"
      subtitle={
        <span className="armada-account__login">
          <UserCheck size={12} strokeWidth={2} aria-hidden="true" />
          {login}
        </span>
      }
      closeLabel="Close panel"
      closeBinding="Esc"
      onClose={account.closeSyncs}
    >
      <ul className="armada-syncs">
        {kinds.map((one) => (
          <li key={one.kind} className="armada-syncs__kind" data-on={one.on ? "" : undefined}>
            <Switch checked={one.on} onChange={(event) => account.setKind(one.kind, event.target.checked)}>
              {KIND[one.kind].label}
            </Switch>
            {!one.on ? null : one.pulling ? (
              <div className="armada-syncs__pulling">
                <Turning said="Pulling" />
              </div>
            ) : (
              <ol className="armada-syncs__items" aria-label={`${KIND[one.kind].label} synced`}>
                {one.items.map((item, at) => (
                  <li key={item.name} className="armada-syncs__item" style={{ "--fill": at } as CSSProperties}>
                    <span className="armada-syncs__name">{item.name}</span>
                    {item.scope === "repo" ? <span className="armada-syncs__repo">{item.repo}</span> : null}
                    <ScopeMark kind={one.kind} item={item} onPress={account.setScope} />
                  </li>
                ))}
              </ol>
            )}
          </li>
        ))}
      </ul>
    </Sheet>
  );
}

function ScopeMark({ kind, item, onPress }: { kind: SyncKindWord; item: SyncItem; onPress: AccountActs["setScope"] }) {
  const { Glyph, said } = SCOPE[item.scope];
  return (
    <Tooltip beside label={said}>
      <button
        type="button"
        className="armada-syncs__scope"
        data-scope={item.scope}
        aria-label={`${said}, ${item.name}`}
        onClick={() => onPress(kind, item.name, nextScope(kind, item.scope))}
      >
        <Glyph size={16} strokeWidth={2} aria-hidden="true" />
      </button>
    </Tooltip>
  );
}
