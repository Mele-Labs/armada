import { ChevronLeft, Menu, Moon, Settings, UserCheck } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { KbdCmd } from "../../primitives/Kbd/Kbd";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { AccountView, accountLogin, useAccount } from "../Account/Account";

/**
 * The title row's one menu: the account, Sleep and Settings behind a single registry glyph, so the
 * row carries four things and not seven (the owner, 14 Oct 2026).
 *
 * **State that matters stays on the button.** Sleep on puts the moon on it, and nothing is signed in
 * puts an attention dot there, so neither needs the menu opened to be read. Off, the row shows no moon.
 *
 * The account entry is where signing in happens: signed out it starts the device flow and its panel
 * shows the code; signed in it opens the account, its profiles and What syncs. Below the narrow bound
 * the row's other controls are listed under these, which is the menu the row had before.
 */
export type TitleMenuProps = {
  /** Sleep mode's toggle. Absent draws no entry. */
  sleep?: { on: boolean; onToggle: () => void };
  /** Opens the Settings page. Absent draws no entry. */
  settings?: { onOpen: () => void; shortcut?: string };
  /** The row's own controls, listed under the entries where the row is narrow. */
  narrow?: ReactNode;
};

export function TitleMenu({ sleep, settings, narrow }: TitleMenuProps) {
  const account = useAccount();
  const [open, setOpen] = useState(false);
  const [view, setView] = useState<"menu" | "account">("menu");
  const root = useRef<HTMLDivElement>(null);
  const login = account === undefined ? undefined : accountLogin(account);
  const signedOut = account !== undefined && login === undefined;
  const choosing = account?.syncs != null;
  const sleeping = sleep?.on === true;

  const away = useRef(() => {});
  away.current = () => {
    setOpen(false);
    setView("menu");
    account?.dismiss();
  };

  // The panel and What syncs share the trailing edge: What syncs opening puts the panel away, and the
  // panel opening puts What syncs away.
  useEffect(() => {
    if (choosing) away.current();
  }, [choosing]);
  const closeSyncs = useRef(account?.closeSyncs);
  closeSyncs.current = account?.closeSyncs;
  useEffect(() => {
    if (open) closeSyncs.current?.();
  }, [open]);

  useEffect(() => {
    if (!open) return undefined;
    function onKey(event: KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      away.current();
    }
    function onDown(event: MouseEvent) {
      if (!root.current?.contains(event.target as Node)) away.current();
    }
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("mousedown", onDown);
    };
  }, [open]);

  if (account === undefined && sleep === undefined && settings === undefined && narrow === undefined) return null;

  const said = ["Menu", ...(sleeping ? ["sleep mode on"] : []), ...(signedOut ? ["signed out"] : [])].join(", ");
  return (
    // Pressing anywhere in the menu is not the first use of Bridge that turns Sleep off.
    <div className="armada-title-bar__menu armada-popover" ref={root} data-keeps-sleep="">
      <span className="armada-popover__trigger">
        <span className="armada-title-menu__button">
          <Button
            variant="ghost"
            size="sm"
            iconOnly
            aria-label="Menu"
            aria-expanded={open}
            title={said}
            onClick={() => {
              if (open) away.current();
              else setOpen(true);
            }}
          >
            <Menu size={16} strokeWidth={2} aria-hidden />
          </Button>
          {sleeping ? (
            <span className="armada-title-menu__moon" aria-hidden>
              <Moon size={12} strokeWidth={2} fill="currentColor" />
            </span>
          ) : null}
          {signedOut ? <span className="armada-title-menu__attention" aria-hidden /> : null}
        </span>
      </span>
      {open ? (
        <div className="armada-popover__panel armada-popover__panel--end armada-title-menu__panel" role="dialog" aria-label="Menu" data-view={view}>
          {view === "account" && account !== undefined ? (
            <div className="armada-title-menu__account">
              <Tooltip beside label="Menu">
                <Button variant="ghost" size="sm" iconOnly aria-label="Back to the menu" onClick={() => setView("menu")}>
                  <ChevronLeft size={16} strokeWidth={2} aria-hidden />
                </Button>
              </Tooltip>
              <AccountView account={account} />
            </div>
          ) : (
            <div className="armada-title-menu__entries">
              {account === undefined ? null : (
                <button
                  type="button"
                  className="armada-title-menu__item"
                  aria-label={login === undefined ? "Sign in with GitHub" : `Account ${login}`}
                  onClick={() => {
                    if (login === undefined) account.signIn();
                    setView("account");
                  }}
                >
                  <span className="armada-title-menu__glyph">
                    <UserCheck size={16} strokeWidth={2} aria-hidden />
                  </span>
                  <span className={login === undefined ? undefined : "armada-title-menu__login"}>{login ?? "Sign in with GitHub"}</span>
                  {signedOut ? <span className="armada-title-menu__attention armada-title-menu__attention--row" aria-hidden /> : null}
                </button>
              )}
              {sleep === undefined ? null : (
                <button type="button" className="armada-title-menu__item" aria-label="Sleep mode" aria-pressed={sleep.on} onClick={sleep.onToggle}>
                  <span className="armada-title-menu__glyph">
                    <Moon size={16} strokeWidth={2} fill={sleep.on ? "currentColor" : "none"} aria-hidden />
                  </span>
                  <span>Sleep</span>
                </button>
              )}
              {settings === undefined ? null : (
                <button
                  type="button"
                  className="armada-title-menu__item"
                  aria-label="Settings"
                  onClick={() => {
                    settings.onOpen();
                    away.current();
                  }}
                >
                  <span className="armada-title-menu__glyph">
                    <Settings size={16} strokeWidth={2} aria-hidden />
                  </span>
                  <span>Settings</span>
                  {settings.shortcut === undefined ? null : <KbdCmd shortcut={settings.shortcut} />}
                </button>
              )}
              {narrow === undefined ? null : (
                <div
                  className="armada-title-menu__narrow"
                  // Any act closes the panel.
                  onClick={(event) => {
                    if ((event.target as Element).closest("button:not(:disabled)") !== null) away.current();
                  }}
                >
                  {narrow}
                </div>
              )}
            </div>
          )}
        </div>
      ) : null}
    </div>
  );
}
