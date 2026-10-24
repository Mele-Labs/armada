import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Ban, Check, CircleDashed, Copy, Database, GitPullRequest, Hammer, House, Laptop, Link as LinkGlyph, Settings, Unplug, UserCheck, UserPen, X } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import type { DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { homePath, useHomeChoice } from "../KitSetup/HomeChoice";
import { ForgeMark } from "./ForgeMark";
import { DEVICE_PAGE, ENDED, SETTING, SIGN_IN_SAID, Turning, accountNeed, useAccount } from "./Account";
import type { Account, AccountFlow, Binding, Profile, SetUpId, SetUpState, UnboundRepository } from "./Account";

/**
 * The account as a viewscreen, the way the Kit's setup and the worktree pool are drawn: a sunken glass under
 * a mono band, one tile for each profile standing on it. **State is the frame**: the active profile in the
 * accent, a sign-in in flight dashed, one that ended in the amber the menu's dot already means, a profile
 * signed out dashed and struck through. The same content is the page in the work area and the panel from the rail.
 *
 * Profiles are always listed, including while another signs in. A sign-in in flight is a tile of its own,
 * and becomes the profile's tile when it finishes.
 */
type Steps = Readonly<Record<SetUpId, SetUpState>>;

const ROWS: readonly { id: SetUpId; label: string; Glyph: typeof House }[] = [
  { id: "identity", label: "Identity", Glyph: UserCheck },
  { id: "store", label: "Store", Glyph: Database },
  { id: "home", label: "Shared home", Glyph: House },
];

type Setup = Extract<AccountFlow, { phase: "setup" }>;

/** What signing in set up for one profile: a row each, a registry glyph and a mark for how far it has got. */
function SetUpRows({ accountId, login, repository, steps, unset, planned }: { accountId: number | undefined; login: string; repository: string; steps: Steps; unset: boolean; planned?: string }) {
  return (
    <ul className="armada-account__rows">
      {ROWS.map(({ id, label, Glyph }) => {
        const state = unset && id === "home" ? "waiting" : steps[id];
        const name =
          id === "identity" ? (accountId === undefined ? "GitHub account" : `GitHub #${accountId}`) : id === "store" ? "This machine's key" : unset ? (planned ?? "") : `${login}/${repository}`;
        return (
          <li key={id} className="armada-account__row" aria-label={label} data-state={state}>
            <Tooltip beside label={label}>
              <span className="armada-account__glyph" role="img" aria-label={label}>
                <Glyph size={16} strokeWidth={2} aria-hidden="true" />
              </span>
            </Tooltip>
            <span className="armada-account__name">{name}</span>
            <span className="armada-account__state">
              {id === "home" && unset ? (
                <Tooltip beside label={`Not set up yet. The first Sync on Kit makes ${planned ?? `${login}/armada-home`}, a private repository, through gh. Every machine signed in as ${login} reads it`}>
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
  );
}

/** Whose it is: this machine's own, or added on purpose. */
function Origin({ chosen }: { chosen: boolean }) {
  const said = chosen ? "Added" : "This machine's own";
  const Glyph = chosen ? UserPen : Laptop;
  return (
    <Tooltip beside label={said}>
      <span className="armada-account__mark" role="img" aria-label={said}>
        <Glyph size={16} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** Puts a notice away: it stays until this is pressed, or until the next sign-in. */
function Dismiss({ account }: { account: Account }) {
  return (
    <Tooltip beside label="Dismiss">
      <Button variant="ghost" size="sm" iconOnly aria-label="Dismiss" onClick={account.dismiss}>
        <X size={16} strokeWidth={2} aria-hidden="true" />
      </Button>
    </Tooltip>
  );
}

/** The profiles a repository could move to, and None, which makes it claim and sync nothing. */
function moveEntries(profiles: readonly Profile[], current: string | null): DropdownMenuEntry[] {
  return [
    ...profiles.map((one): DropdownMenuEntry => ({ kind: "item", id: one.login, label: one.login, selected: one.login === current })),
    { kind: "separator", id: "rule" },
    { kind: "item", id: NONE, label: "None", selected: current === null },
  ];
}

const NONE = "none";

/** How a repository came to be bound: its owner is the account or one of its organisations, or the person chose it. */
function Bound({ by }: { by: Binding }) {
  const said = by === "matched" ? "Matched" : "Chosen";
  const Glyph = by === "matched" ? UserCheck : UserPen;
  return (
    <Tooltip beside label={said}>
      <span className="armada-account__mark" role="img" aria-label={said}>
        <Glyph size={16} strokeWidth={2} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** The repositories bound to one profile, scrolling inside the tile where there are many. A row's menu moves it. */
function Repositories({ account, profile }: { account: Account; profile: Profile }) {
  const { bind } = account;
  const rows = profile.repositories ?? [];
  if (rows.length === 0) return null;
  return (
    <ul className="armada-account__repos" aria-label={`Repositories of ${profile.login}`}>
      {rows.map(({ root, name, by }) => (
        <li key={root} className="armada-account__repo" aria-label={name}>
          <span className="armada-account__name">{name}</span>
          <Bound by={by} />
          {bind === undefined ? null : (
            <DropdownMenu
              triggerLabel="Move"
              portal
              triggerAriaLabel={`Move ${name}`}
              entries={moveEntries(account.profiles, profile.login)}
              onSelect={(id) => bind(root, id === NONE ? null : id)}
            />
          )}
        </li>
      ))}
    </ul>
  );
}

/** One profile: its login and whose it is, its set-up rows, the repositories bound to it, and the acts on it. */
function ProfileTile({ account, profile, setup }: { account: Account; profile: Profile; setup: Setup | undefined }) {
  const noHome = account.home === false;
  const steps: Steps = setup?.steps ?? { identity: "done", store: "done", home: profile.repository === undefined ? "waiting" : "done" };
  const repository = setup === undefined ? (profile.repository ?? "") : setup.repository;
  // A home with no repository named does not exist yet, whatever the step says.
  const unset = repository === "";
  const [choice] = useHomeChoice(profile.login);
  return (
    <li className="armada-account__tile" data-tile="profile" aria-label={profile.login}>
      <header className="armada-account__tile-head">
        <UserCheck size={16} strokeWidth={2} aria-hidden="true" />
        <span className="armada-account__login">{profile.login}</span>
        <Origin chosen={profile.chosen} />
      </header>
      <SetUpRows accountId={profile.account_id} login={profile.login} repository={repository} steps={steps} unset={unset} planned={homePath(choice)} />
      <Repositories account={account} profile={profile} />
      <footer className="armada-account__tile-foot">
        {!profile.chosen ? null : !noHome ? (
          <Button variant="ghost" size="sm" onClick={() => account.choose(profile.login)}>
            What syncs
          </Button>
        ) : (
          <Tooltip beside label="No shared home yet. Press Sync on Kit to make it, then choose what syncs here">
            <Button variant="ghost" size="sm" disabled>
              What syncs
            </Button>
          </Tooltip>
        )}
        <Button variant="ghost" size="sm" aria-label={`Sign out ${profile.login}`} onClick={() => account.signOut(profile.login)}>
          Sign out
        </Button>
      </footer>
    </li>
  );
}

/** A login that has signed in and is still being set up, before Fleet lists it. */
function PendingTile({ setup, first }: { setup: Setup; first: boolean }) {
  return (
    <li className="armada-account__tile" data-tile="profile" aria-label={setup.login}>
      <header className="armada-account__tile-head">
        <UserCheck size={16} strokeWidth={2} aria-hidden="true" />
        <span className="armada-account__login">{setup.login}</span>
        <Origin chosen={!first} />
      </header>
      <SetUpRows accountId={undefined} login={setup.login} repository={setup.repository} steps={setup.steps} unset={setup.repository === "" && setup.steps.home !== "done"} />
    </li>
  );
}

/** Repositories no profile owns, or two both match: each waits for a choice, or has been given None. */
function NotMatched({ account, rows }: { account: Account; rows: readonly UnboundRepository[] }) {
  const { bind } = account;
  return (
    <Frame label="Not matched">
      <div className="armada-account-screen__glass">
        <div className="armada-account__tile" data-tile="plan">
          <ul className="armada-account__repos" aria-label="Not matched">
            {rows.map(({ root, name, none }) => {
              const said = none === true ? "None" : "Waiting for a choice";
              const Glyph = none === true ? Ban : CircleDashed;
              return (
                <li key={root} className="armada-account__repo" aria-label={name}>
                  <span className="armada-account__name">{name}</span>
                  <Tooltip beside label={said}>
                    <span className="armada-account__unset" role="img" aria-label={said}>
                      <Glyph size={16} strokeWidth={2} aria-hidden="true" />
                    </span>
                  </Tooltip>
                  {bind === undefined ? null : (
                    <DropdownMenu
                      triggerLabel="Choose"
                      portal
                      triggerAriaLabel={`Choose for ${name}`}
                      entries={moveEntries(account.profiles, null).map((entry) =>
                        entry.kind === "item" && entry.id === NONE && none !== true ? { ...entry, selected: false } : entry,
                      )}
                      onSelect={(id) => bind(root, id === NONE ? null : id)}
                    />
                  )}
                </li>
              );
            })}
          </ul>
        </div>
      </div>
    </Frame>
  );
}

/** The time a code has left, counted down from the reading it came with. */
function Expiry({ from }: { from: number }) {
  const [left, setLeft] = useState(from);
  useEffect(() => {
    setLeft(from);
    const timer = window.setInterval(() => setLeft((was) => Math.max(0, was - 1)), 1000);
    return () => window.clearInterval(timer);
  }, [from]);
  const minutes = Math.floor(left / 60);
  const seconds = String(left % 60).padStart(2, "0");
  return (
    <Tooltip beside label="Expires in">
      <span className="armada-account__expiry">{`${minutes}:${seconds}`}</span>
    </Tooltip>
  );
}

/** The head every tile for a profile being added wears: the label, and what the head's end carries. */
function AddingHead({ children }: { children?: ReactNode }) {
  return (
    <header className="armada-account__tile-head">
      <UserCheck size={16} strokeWidth={2} aria-hidden="true" />
      <span className="armada-account__adding">Adding</span>
      <span className="armada-account__tile-end">{children}</span>
    </header>
  );
}

/** A sign-in in flight: the address as a link, the code large with a copy act, a waiting mark, the expiry, and Cancel. */
function CodeTile({ account, code, expiresIn }: { account: Account; code: string; expiresIn: number | undefined }) {
  const need = accountNeed(account);
  const [copied, setCopied] = useState(false);
  const timer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(timer.current), []);
  const copy = () => {
    void Promise.resolve(navigator.clipboard?.writeText(code)).catch(() => undefined);
    setCopied(true);
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setCopied(false), 1500);
  };
  return (
    <li className="armada-account__tile" data-tile="adding" aria-label="Adding">
      <AddingHead>
        {expiresIn === undefined ? null : <Expiry key={code} from={expiresIn} />}
        {need === undefined ? null : <Turning said={need.said} />}
      </AddingHead>
      <div className="armada-account__tile-body">
        <button type="button" className="armada-account__link" aria-label={`Open ${DEVICE_PAGE}`} onClick={account.openDevicePage}>
          <LinkGlyph size={16} strokeWidth={2} aria-hidden="true" />
          <span>{DEVICE_PAGE}</span>
        </button>
        <div className="armada-account__code-row">
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
          <Tooltip beside label={copied ? "Copied" : "Copy"}>
            <Button variant="ghost" size="sm" iconOnly aria-label="Copy code" onClick={copy}>
              {copied ? <Check size={16} strokeWidth={2} aria-hidden="true" /> : <Copy size={16} strokeWidth={2} aria-hidden="true" />}
            </Button>
          </Tooltip>
        </div>
      </div>
      {account.cancel === undefined ? null : (
        <footer className="armada-account__tile-foot">
          <Button variant="ghost" size="sm" onClick={account.cancel}>
            Cancel
          </Button>
        </footer>
      )}
    </li>
  );
}

/** A sign-in that ended without signing in: Fleet's sentence beside the mark for how, and Try again. */
function EndedTile({ account, ended }: { account: Account; ended: NonNullable<Account["ended"]> }) {
  const { Glyph, said } = ENDED[ended.outcome];
  return (
    <li className="armada-account__tile" data-tile="adding" data-state="ended" aria-label="Sign-in ended">
      <AddingHead>
        <Dismiss account={account} />
      </AddingHead>
      <div className="armada-account__tile-body">
        <div className="armada-account__said-row">
          <Tooltip beside label={said}>
            <span className="armada-account__glyph" role="img" aria-label={said}>
              <Glyph size={16} strokeWidth={2} aria-hidden="true" />
            </span>
          </Tooltip>
          <span className="armada-account__said">{ended.said}</span>
        </div>
      </div>
      <footer className="armada-account__tile-foot">
        <Button variant="secondary" size="sm" ground="sunken" onClick={account.signIn}>
          Try again
        </Button>
      </footer>
    </li>
  );
}

/** Sign-in refused for a blank setting: Fleet's sentence, and a button to each blank setting in Settings. */
function UnavailableTile({
  account,
  unavailable,
  onSettings,
}: {
  account: Account;
  unavailable: NonNullable<Account["unavailable"]>;
  onSettings: (() => void) | undefined;
}) {
  return (
    <li className="armada-account__tile" data-tile="adding" data-state="ended" aria-label="Sign-in unavailable">
      <AddingHead>
        <Dismiss account={account} />
      </AddingHead>
      <div className="armada-account__tile-body">
        <div className="armada-account__said-row">
          <Tooltip beside label="Not set up">
            <span className="armada-account__glyph" role="img" aria-label="Not set up">
              <CircleDashed size={16} strokeWidth={2} aria-hidden="true" />
            </span>
          </Tooltip>
          <span className="armada-account__said">{unavailable.said}</span>
        </div>
      </div>
      {onSettings === undefined ? null : (
        <footer className="armada-account__tile-foot">
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
        </footer>
      )}
    </li>
  );
}

/** A profile signed out: its login struck through, and the store's row named only where it stays. */
function LeftTile({ account, note }: { account: Account; note: NonNullable<Account["left"]> }) {
  return (
    <li className="armada-account__tile" data-tile="left" aria-label="Signed out">
      <header className="armada-account__tile-head">
        <UserCheck size={16} strokeWidth={2} aria-hidden="true" />
        <span className="armada-account__login">{note.login}</span>
        <span className="armada-account__tile-end">
          <Dismiss account={account} />
        </span>
      </header>
      {!note.storeRowRemains ? null : (
        <div className="armada-account__tile-body">
          <div className="armada-account__said-row">
            <Tooltip beside label={note.said ?? "Store not reached"}>
              <span className="armada-account__glyph" role="img" aria-label="Store not reached">
                <Unplug size={16} strokeWidth={2} aria-hidden="true" />
              </span>
            </Tooltip>
            <span className="armada-account__said">Store still lists this device</span>
          </div>
        </div>
      )}
    </li>
  );
}

/** What signing in turns on, and what the machine holds of it once it has: three things, off then on. */
const TURNS_ON: readonly { id: "claims" | "owners" | "home"; name: string; Glyph: typeof House; said: string }[] = [
  { id: "claims", name: "Red main claims", Glyph: Hammer, said: "See when your other machine is fixing a red main, and it sees yours" },
  { id: "owners", name: "Pull request owners", Glyph: GitPullRequest, said: "See which Job owned a pull request, from any machine" },
  { id: "home", name: "Shared home", Glyph: House, said: "Sync skills, agents and instructions, per kind and per machine" },
];

/** What signing in will set up, drawn as the set-up rows will be, none of them yet. */
const WILL_SET_UP: readonly { id: SetUpId; label: string; said: string; Glyph: typeof House }[] = [
  { id: "identity", label: "Identity", said: "GitHub account", Glyph: UserCheck },
  { id: "store", label: "Store", said: "This machine's key", Glyph: Database },
  { id: "home", label: "Shared home", said: "<login>/armada-home, private", Glyph: House },
];

/** What signing in never does, a fact each. */
const NEVER: readonly string[] = [
  "Nothing is written to your repositories or pull requests",
  "Your token is not kept",
  "A work machine syncs nothing until you turn a kind on",
];

/** Which of the three the machine holds now: the store once a profile exists, the home once it is made. */
function turnedOn(account: Account, setup: Setup | undefined): Readonly<Record<"claims" | "owners" | "home", boolean>> {
  const signedIn = account.profiles.length > 0;
  const store = signedIn || setup?.steps.store === "done";
  const home = account.home !== false && (signedIn || setup?.steps.home === "done");
  return { claims: store, owners: store, home };
}

/** One frame on the glass: a mono band over a ruled grid, the way the pool is drawn. */
function Frame({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="armada-account-screen__view" aria-label={label}>
      <header className="armada-account-screen__band">{label}</header>
      {children}
    </section>
  );
}

/** The three things sign-in turns on, each a tile: dashed and dim while off, framed and lit once on. */
function TurnsOnTiles({ on }: { on: Readonly<Record<"claims" | "owners" | "home", boolean>> }) {
  return (
    <ul className="armada-account-screen__glass" aria-label="Turns on">
      {TURNS_ON.map(({ id, name, Glyph, said }) => (
        <li key={id} className="armada-account__tile" data-tile="feature" data-on={on[id] ? "" : undefined} aria-label={`${name}, ${on[id] ? "on" : "off"}`}>
          <header className="armada-account__tile-head">
            <Glyph size={16} strokeWidth={2} aria-hidden="true" />
            <span className="armada-account__login">{name}</span>
            <span className="armada-account__tile-end">
              <Tooltip beside label={on[id] ? "On" : "Off"}>
                <span className={on[id] ? "armada-account__done" : "armada-account__unset"} role="img" aria-label={on[id] ? "On" : "Off"}>
                  {on[id] ? <Check size={16} strokeWidth={2} aria-hidden="true" /> : <CircleDashed size={16} strokeWidth={2} aria-hidden="true" />}
                </span>
              </Tooltip>
            </span>
          </header>
          <p className="armada-account__fact">{said}</p>
        </li>
      ))}
    </ul>
  );
}

/** What signing in never does: on the page before and after, since it is true of both. */
function Never() {
  return (
    <Frame label="Never">
      <div className="armada-account-screen__glass">
        <div className="armada-account__tile" data-tile="plan">
          <ul className="armada-account__rows">
            {NEVER.map((said) => (
              <li key={said} className="armada-account__row" data-state="waiting">
                <Tooltip beside label="Never">
                  <span className="armada-account__glyph" role="img" aria-label="Never">
                    <Ban size={16} strokeWidth={2} aria-hidden="true" />
                  </span>
                </Tooltip>
                <span className="armada-account__name">{said}</span>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </Frame>
  );
}

/** The way to a second machine, one row a step. Step 1 and 2 read the account; step 3 happens on the other machine, so it never reads done. */
const SYNC_STEPS: readonly { id: string; name: string; why: string; Glyph: typeof House }[] = [
  { id: "sign-in", name: "Sign in here", why: "The same GitHub account on each machine is what ties them together", Glyph: UserCheck },
  { id: "sync", name: "Sync on Kit", why: "Choose what syncs, then press Sync: the first one makes your private armada-home repository", Glyph: House },
  { id: "add", name: "Add to this machine", why: "On the other machine, sign in as the same account, open Kit and press this on each row", Glyph: Laptop },
];

/** Sync your Kit: the three steps, the next undone one carrying the button. Drawn signed out too, so the reason comes before the sign-in. */
function SyncGuide({ account, onKit }: { account: Account; onKit: (() => void) | undefined }) {
  const signedIn = account.profiles.length > 0;
  const homeMade = signedIn && account.home !== false;
  const [choice] = useHomeChoice(account.profiles[0]?.login ?? "");
  const made = account.profiles[0]?.repository;
  const sync = homeMade
    ? `The home is ${account.profiles[0]?.login}/${made ?? "armada-home"}. Press Sync on Kit to push or pull`
    : `Choose what syncs, then press Sync: the first one makes ${signedIn ? homePath(choice) : "<login>/armada-home"}, a private repository`;
  const done = [signedIn, homeMade, false] as const;
  const next = done.findIndex((one) => !one);
  return (
    <Frame label="Sync your Kit">
      <div className="armada-account-screen__glass">
        <div className="armada-account__tile" data-tile="plan">
          <ul className="armada-account__rows">
            {SYNC_STEPS.map(({ id, name, why: said, Glyph }, at) => {
              const why = at === 1 ? sync : said;
              return (
              <li key={id} className="armada-account__row" aria-label={name} data-state={done[at] || at === next ? "done" : "waiting"}>
                <Tooltip beside label={name}>
                  <span className="armada-account__glyph" role="img" aria-label={name}>
                    <Glyph size={16} strokeWidth={2} aria-hidden="true" />
                  </span>
                </Tooltip>
                <span className="armada-account__step">
                  <span className="armada-account__name">{name}</span>
                  <span className="armada-account__why">{why}</span>
                </span>
                <span className="armada-account__state">
                  {at === 1 && next === 1 && onKit !== undefined ? (
                    <Button variant="secondary" size="sm" ground="sunken" onClick={onKit}>
                      Open Kit
                    </Button>
                  ) : done[at] ? (
                    <Tooltip beside label="Done">
                      <span className="armada-account__done" role="img" aria-label="Done">
                        <Check size={16} strokeWidth={2} aria-hidden="true" />
                      </span>
                    </Tooltip>
                  ) : (
                    <Tooltip beside label="Not yet">
                      <span className="armada-account__unset" role="img" aria-label="Not yet">
                        <CircleDashed size={16} strokeWidth={2} aria-hidden="true" />
                      </span>
                    </Tooltip>
                  )}
                </span>
              </li>
              );
            })}
          </ul>
        </div>
      </div>
    </Frame>
  );
}

/** Before signing in: the rows set-up will show, none yet. */
function BeforeSignIn() {
  return (
    <>
      <Frame label="Sets up">
        <div className="armada-account-screen__glass">
          <div className="armada-account__tile" data-tile="plan">
            <ul className="armada-account__rows">
              {WILL_SET_UP.map(({ id, label, said, Glyph }) => (
                <li key={id} className="armada-account__row" aria-label={label} data-state="waiting">
                  <Tooltip beside label={label}>
                    <span className="armada-account__glyph" role="img" aria-label={label}>
                      <Glyph size={16} strokeWidth={2} aria-hidden="true" />
                    </span>
                  </Tooltip>
                  <span className="armada-account__name">{said}</span>
                  <span className="armada-account__state">
                    <Tooltip beside label="Not yet">
                      <span className="armada-account__unset" role="img" aria-label="Not yet">
                        <CircleDashed size={16} strokeWidth={2} aria-hidden="true" />
                      </span>
                    </Tooltip>
                  </span>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </Frame>
    </>
  );
}

/**
 * The page: profiles on the glass, and under them what signing in turns on, drawn off and then on, with
 * what it will set up and never do while signed out. The band is the page's header where `band` is set.
 * `onSettings` opens Settings for the buttons that name a blank setting; without it they are not drawn.
 */
export function AccountView({ account, onSettings, onKit, band = false, machine }: { account: Account; onSettings?: () => void; onKit?: () => void; band?: boolean; machine?: string | undefined }) {
  const { profiles, flow, ended, unavailable, left } = account;
  const setup = flow.phase === "setup" ? flow : undefined;
  const pending = setup !== undefined && !profiles.some((one) => one.login === setup.login) ? setup : undefined;
  const failed = ended !== undefined || unavailable !== undefined;
  const before = profiles.length === 0 && pending === undefined;
  const signedOut = before && flow.phase !== "code" && !failed;
  const name = machine ?? account.machine;
  return (
    <section className="armada-account-screen" aria-label="Account" data-page={band ? "" : undefined}>
      {!band ? null : (
        <header className="armada-account-screen__head">
          <h1 className="armada-account-screen__title">Account</h1>
          {name === undefined ? null : (
            <Tooltip beside label="This machine">
              <p className="armada-account-screen__where">
                <Laptop size={16} strokeWidth={2} aria-hidden="true" />
                <span>{name}</span>
              </p>
            </Tooltip>
          )}
        </header>
      )}
      <Frame label="Profiles">
        <ul className="armada-account-screen__glass armada-account-screen__glass--profiles" aria-label="Profiles">
          {profiles.map((one) => (
            <ProfileTile
              key={one.login}
              account={account}
              profile={one}
              setup={setup !== undefined && setup.login === one.login ? setup : undefined}
            />
          ))}
          {pending === undefined ? null : <PendingTile setup={pending} first={profiles.length === 0} />}
          {left === undefined ? null : <LeftTile account={account} note={left} />}
          {flow.phase !== "code" ? null : <CodeTile account={account} code={flow.code} expiresIn={flow.expiresIn} />}
          {flow.phase === "code" || ended === undefined ? null : <EndedTile account={account} ended={ended} />}
          {flow.phase === "code" || unavailable === undefined ? null : <UnavailableTile account={account} unavailable={unavailable} onSettings={onSettings} />}
          {signedOut ? (
            <li className="armada-account__tile" data-tile="empty" aria-label="Signed out, no profile">
              <div className="armada-account__tile-body armada-account__tile-body--centered">
                <Button variant="secondary" size="sm" ground="sunken" onClick={account.signIn}>
                  <ForgeMark />
                  {SIGN_IN_SAID}
                </Button>
              </div>
            </li>
          ) : null}
          {profiles.length === 0 || flow.phase === "code" || failed ? null : (
            <li className="armada-account__tile" data-tile="add" aria-label="Add">
              <div className="armada-account__tile-body armada-account__tile-body--centered">
                <Button variant="secondary" size="sm" ground="sunken" onClick={account.addProfile}>
                  Add profile
                </Button>
              </div>
            </li>
          )}
        </ul>
      </Frame>
      {account.unbound === undefined || account.unbound.length === 0 ? null : <NotMatched account={account} rows={account.unbound} />}
      <SyncGuide account={account} onKit={onKit} />
      <Frame label="Turns on">
        <TurnsOnTiles on={turnedOn(account, setup)} />
      </Frame>
      {before ? <BeforeSignIn /> : null}
      <Never />
    </section>
  );
}

/** The account as a page in the work area, the way Settings opens. Nothing where no account is provided. */
export function AccountPage({ onSettings, onKit, machine }: { onSettings?: () => void; onKit?: () => void; machine?: string | undefined }) {
  const account = useAccount();
  if (account === undefined) return null;
  return <AccountView account={account} band machine={machine} {...(onSettings === undefined ? {} : { onSettings })} {...(onKit === undefined ? {} : { onKit })} />;
}
