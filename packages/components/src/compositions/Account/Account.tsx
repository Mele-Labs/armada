import { createContext, useContext } from "react";
import type { CSSProperties, ReactNode } from "react";
import {
  Ban,
  CircleDashed,
  Clock,
  FolderGit2,
  Globe,
  Laptop,
  RotateCw,
  Unplug,
  X,
  type LucideIcon,
} from "lucide-react";

import { Sheet } from "../../primitives/Sheet/Sheet";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SharedHomeChip } from "../KitSetup/SharedHome";

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
  | "global_instructions"
  | "sub_agents"
  | "commands"
  | "mcp_servers"
  | "allowlist"
  | "workflows"
  | "memory"
  | "mods";

export type SyncItem = { name: string; scope: Scope; repo: string };

/** One kind in What syncs: off until turned on, pulling while it fills in, then its items. */
export type SyncChoice = { kind: SyncKindWord; on: boolean; pulling: boolean; items: readonly SyncItem[] };

export type Profile = {
  /** The forge's numeric id, which names the profile in every act on it. */
  account_id: number;
  login: string;
  /** The wire still carries it; no profile is drawn as the active one, since each repository is bound to one profile. */
  active: boolean;
  /** Added on purpose beside the machine's own: what it syncs is the person's choice. */
  chosen: boolean;
  /** The repository of the shared home it set up, once there is one. */
  repository?: string;
  /** Where its shared home is, or will be made: the owner (the login or an organisation) and the name, and whether this machine holds it. Absent falls back to the choice kept on this window. */
  home?: { account: string; repository: string; made: boolean };
  /** The organisations the login belongs to, read at sign-in: the owners a shared home can be made under, beside the login. */
  orgs?: readonly string[];
  /** The repositories bound to this profile, each marked for how it came to be. */
  repositories?: readonly BoundRepository[];
};

/** How a repository came to be bound: its owner is the account or one of its organisations, or the person chose it. */
export type Binding = "matched" | "chosen";
/** `root` names the repository in every act and is never drawn: two checkouts can share a folder name. */
export type BoundRepository = { root: string; name: string; by: Binding };

/** A repository no profile owns: no profile matched it, or two both did. `none` once the person chose that it claims and syncs nothing. */
export type UnboundRepository = { root: string; name: string; none?: boolean };

export type SetUpId = "identity" | "store" | "home";
export type SetUpState = "waiting" | "working" | "done";

export type AccountFlow =
  | { phase: "idle" }
  /** The device code, shown until it is typed at GitHub. `expiresIn` is the seconds it had left at this reading. */
  | { phase: "code"; code: string; expiresIn?: number }
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

export type AccountRead = {
  /** Empty is signed out. */
  profiles: readonly Profile[];
  flow: AccountFlow;
  /** The profile whose What syncs is open. */
  syncs: { login: string; kinds: readonly SyncChoice[]; home: { account?: string; repository: string; made: boolean } } | null;
  /** The last sign-in ended without signing in. Absent while one is in flight or none has ended. */
  ended?: SignInEnd;
  /** Starting a sign-in was refused for a blank setting. */
  unavailable?: SignInUnavailable;
  /** A profile signed out. */
  left?: SignedOutNote;
  /** Whether the shared home exists. Absent is that it does. */
  home?: boolean;
  /** This machine's name, beside the page's title. */
  machine?: string;
  /** Repositories on this machine that wait for a choice. Absent or empty draws nothing. */
  unbound?: readonly UnboundRepository[];
};

export type AccountActs = {
  /** Start the device flow. */
  signIn: () => void;
  /** The link on the code card: GitHub's page, where the code is typed. */
  openDevicePage: () => void;
  /** Start the device flow for another login. */
  addProfile: () => void;
  /** Put a notice away: what ended, what was refused, who signed out. */
  dismiss: () => void;
  /** Give up the sign-in waiting on its code. Absent where Fleet has no such act. */
  cancel?: () => void;
  switchTo: (login: string) => void;
  /** Binds the repository at `root` to a profile, or to none (`null`). Absent where Fleet serves no repositories. */
  bind?: (root: string, login: string | null) => void;
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
  global_instructions: { label: "Global instructions", scopes: ["global", "private"] },
  sub_agents: { label: "Sub agents", scopes: ALL_SCOPES },
  commands: { label: "Commands", scopes: ALL_SCOPES },
  mcp_servers: { label: "MCP servers", scopes: ALL_SCOPES },
  allowlist: { label: "Allowlist", scopes: ALL_SCOPES },
  workflows: { label: "Workflows", scopes: ALL_SCOPES },
  memory: { label: "Memory", scopes: ALL_SCOPES },
  // A mod is the machine's, read from Bridge's mod folder: it travels everywhere or stays here.
  mods: { label: "Mods", scopes: ["global", "private"] },
};

/** The kinds in the order What syncs lists them. */
export const SYNC_KINDS: readonly SyncKindWord[] = [
  "skills",
  "plugins",
  "global_instructions",
  "sub_agents",
  "commands",
  "mcp_servers",
  "allowlist",
  "workflows",
  "memory",
  "mods",
];

/** The address the code is typed at. */
export const DEVICE_PAGE = "github.com/login/device";

/** The one act signed out has. */
export const SIGN_IN_SAID = "Sign in with GitHub";

/** The mark on a waiting or in-progress row: the turn is the only motion. */
export function Turning({ said }: { said: string }) {
  return (
    <Tooltip beside label={said}>
      <span className="armada-account__turn" role="img" aria-label={said}>
        <RotateCw size={16} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
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

/** One glyph per way a sign-in ended, and the word its tooltip gives it. */
export const ENDED = {
  denied: { Glyph: Ban, said: "Turned down" },
  expired: { Glyph: Clock, said: "Expired" },
  unreachable: { Glyph: Unplug, said: "Not reached" },
  refused: { Glyph: X, said: "Refused" },
} as const;

/** The setting each blank category names, as Settings calls it. */
export const SETTING: Record<BlankSetting, string> = { account: "Sign-in app", store: "Store address" };

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
  const { login, kinds, home } = account.syncs;
  return (
    <Sheet
      kind="what-syncs"
      open
      floating
      title="What syncs"
      subtitle={<SharedHomeChip account={home.account ?? login} repository={home.repository} made={home.made} />}
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
