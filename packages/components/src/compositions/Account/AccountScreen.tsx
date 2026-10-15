import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { Ban, Check, CircleDashed, Copy, Database, GitPullRequest, Hammer, House, Laptop, Link as LinkGlyph, Settings, Unplug, UserCheck, UserPen, X } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { DEVICE_PAGE, ENDED, SETTING, SIGN_IN_SAID, Turning, accountNeed, useAccount, useAccountPanel } from "./Account";
import type { Account, AccountFlow, Profile, SetUpId, SetUpState } from "./Account";

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
function SetUpRows({ accountId, login, repository, steps, unset }: { accountId: number | undefined; login: string; repository: string; steps: Steps; unset: boolean }) {
  return (
    <ul className="armada-account__rows">
      {ROWS.map(({ id, label, Glyph }) => {
        const state = unset && id === "home" ? "waiting" : steps[id];
        const name =
          id === "identity" ? (accountId === undefined ? "GitHub account" : `GitHub #${accountId}`) : id === "store" ? "This machine's key" : unset ? "" : `${login}/${repository}`;
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

function Active() {
  return (
    <Tooltip beside label="Active">
      <span className="armada-account__active" role="img" aria-label="Active">
        <Check size={16} strokeWidth={2} aria-hidden="true" />
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

/** One profile: its login, whose it is and whether it is active, its set-up rows, and the acts on it. */
function ProfileTile({ account, profile, setup, only }: { account: Account; profile: Profile; setup: Setup | undefined; only: boolean }) {
  const noHome = account.home === false;
  const steps: Steps = setup?.steps ?? { identity: "done", store: "done", home: noHome && profile.repository === undefined ? "waiting" : "done" };
  const repository = setup === undefined ? (profile.repository ?? "") : setup.repository;
  const unset = repository === "" && steps.home !== "done";
  return (
    <li className="armada-account__tile" data-tile="profile" data-active={profile.active ? "" : undefined} aria-label={profile.login}>
      <header className="armada-account__tile-head">
        <UserCheck size={16} strokeWidth={2} aria-hidden="true" />
        <span className="armada-account__login">{profile.login}</span>
        <Origin chosen={profile.chosen} />
        {profile.active ? <Active /> : null}
      </header>
      <SetUpRows accountId={profile.account_id} login={profile.login} repository={repository} steps={steps} unset={unset} />
      <footer className="armada-account__tile-foot">
        {!profile.chosen ? null : !noHome ? (
          <Button variant="ghost" size="sm" onClick={() => account.choose(profile.login)}>
            What syncs
          </Button>
        ) : account.syncsWithoutHome === "hidden" ? null : (
          <Tooltip beside label="No shared home yet">
            <Button variant="ghost" size="sm" disabled>
              What syncs
            </Button>
          </Tooltip>
        )}
        {only || profile.active ? null : (
          <Button variant="ghost" size="sm" aria-label={`Make ${profile.login} active`} onClick={() => account.switchTo(profile.login)}>
            Make active
          </Button>
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
    <li className="armada-account__tile" data-tile="profile" data-active={first ? "" : undefined} aria-label={setup.login}>
      <header className="armada-account__tile-head">
        <UserCheck size={16} strokeWidth={2} aria-hidden="true" />
        <span className="armada-account__login">{setup.login}</span>
        <Origin chosen={!first} />
        {first ? <Active /> : null}
      </header>
      <SetUpRows accountId={undefined} login={setup.login} repository={setup.repository} steps={setup.steps} unset={setup.repository === "" && setup.steps.home !== "done"} />
    </li>
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
  { id: "owners", name: "Pull request owners", Glyph: GitPullRequest, said: "See which Session or Job owned a pull request, from any machine" },
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

/** Before signing in: the rows set-up will show, none yet, and the lines it never crosses. */
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
    </>
  );
}

/**
 * The page: profiles on the glass, and under them what signing in turns on, drawn off and then on, with
 * what it will set up and never do while signed out. The band is the page's header where `band` is set.
 * `onSettings` opens Settings for the buttons that name a blank setting; without it they are not drawn.
 */
export function AccountView({ account, onSettings, band = false, machine }: { account: Account; onSettings?: () => void; band?: boolean; machine?: string | undefined }) {
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
        <ul className="armada-account-screen__glass" aria-label="Profiles">
          {profiles.map((one) => (
            <ProfileTile
              key={one.login}
              account={account}
              profile={one}
              setup={setup !== undefined && setup.login === one.login ? setup : undefined}
              only={profiles.length < 2}
            />
          ))}
          {pending === undefined ? null : <PendingTile setup={pending} first={profiles.length === 0} />}
          {left === undefined ? null : <LeftTile account={account} note={left} />}
          {flow.phase !== "code" ? null : <CodeTile account={account} code={flow.code} expiresIn={flow.expiresIn} />}
          {flow.phase === "code" || ended === undefined ? null : <EndedTile account={account} ended={ended} />}
          {flow.phase === "code" || unavailable === undefined ? null : <UnavailableTile account={account} unavailable={unavailable} onSettings={onSettings} />}
          {signedOut ? (
            <li className="armada-account__tile" data-tile="empty" aria-label="Signed out, no profile">
              <div className="armada-account__tile-body" data-centered="">
                <Button variant="primary" size="sm" onClick={account.signIn}>
                  {SIGN_IN_SAID}
                </Button>
              </div>
            </li>
          ) : null}
          {profiles.length === 0 || flow.phase === "code" || failed ? null : (
            <li className="armada-account__tile" data-tile="add" aria-label="Add">
              <div className="armada-account__tile-body" data-centered="">
                <Button variant="secondary" size="sm" ground="sunken" onClick={account.addProfile}>
                  Add profile
                </Button>
              </div>
            </li>
          )}
        </ul>
      </Frame>
      <Frame label="Turns on">
        <TurnsOnTiles on={turnedOn(account, setup)} />
      </Frame>
      {before ? <BeforeSignIn /> : null}
    </section>
  );
}

/** The account as a page in the work area, the way Settings opens. Nothing where no account is provided. */
export function AccountPage({ onSettings, machine }: { onSettings?: () => void; machine?: string | undefined }) {
  const account = useAccount();
  if (account === undefined) return null;
  return <AccountView account={account} band machine={machine} {...(onSettings === undefined ? {} : { onSettings })} />;
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
      {...(account.machine === undefined ? {} : { subtitle: <span className="armada-account-screen__where">{account.machine}</span> })}
      closeLabel="Close panel"
      closeBinding="Esc"
      onClose={() => setOpen(false)}
    >
      <div className="armada-account-panel">
        <AccountView
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
