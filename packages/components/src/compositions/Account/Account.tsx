import { createContext, useContext } from "react";
import type { CSSProperties, ReactNode } from "react";
import { Check, Database, FolderGit2, Globe, House, Laptop, Link as LinkGlyph, RotateCw, UserCheck } from "lucide-react";

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

export type AccountRead = {
  /** Empty is signed out. */
  profiles: readonly Profile[];
  flow: AccountFlow;
  /** The profile whose What syncs is open. */
  syncs: { login: string; kinds: readonly SyncChoice[] } | null;
};

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
};

export type Account = AccountRead & AccountActs;

const Held = createContext<Account | undefined>(undefined);

export function AccountProvider({ value, children }: { value: Account; children: ReactNode }) {
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

/** The account's own content: the device code while signing in, what it set up, then the profiles. Absent where there is nothing to show. */
export function AccountView({ account }: { account: Account }) {
  const here = account.profiles.find((one) => one.active) ?? account.profiles[0];
  if (account.flow.phase === "code") return <DeviceCode code={account.flow.code} account={account} />;
  if (account.flow.phase === "setup") return <SetUp flow={account.flow} />;
  return here === undefined ? null : <Profiles account={account} />;
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
          return (
            <li key={id} className="armada-account__row" aria-label={label} data-state={state}>
              <Tooltip beside label={label}>
                <span className="armada-account__glyph" role="img" aria-label={label}>
                  <Glyph size={16} strokeWidth={2} aria-hidden="true" />
                </span>
              </Tooltip>
              <span className="armada-account__name">
                {id === "store" ? "Store" : flow.login}
                {id === "home" ? <span className="armada-account__repo">/{flow.repository}</span> : null}
              </span>
              <span className="armada-account__state">
                {state === "working" ? (
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

/** The profiles this machine has joined, one chip each, which is active marked, and the act to add one. */
function Profiles({ account }: { account: Account }) {
  return (
    <section className="armada-account__card" aria-label="Profiles">
      {/* One profile is the account chip alone; the chips draw once a second joins. */}
      {account.profiles.length < 2 ? null : (
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
            {one.chosen ? (
              <Button variant="ghost" size="sm" onClick={() => account.choose(one.login)}>
                What syncs
              </Button>
            ) : null}
          </li>
        ))}
      </ul>
      )}
      <div className="armada-account__foot" data-alone={account.profiles.length < 2 ? "" : undefined}>
        <Button variant="secondary" size="sm" ground="sunken" onClick={account.addProfile}>
          Add profile
        </Button>
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
