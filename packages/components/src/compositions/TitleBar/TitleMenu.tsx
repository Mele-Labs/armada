import { Menu, Moon, Settings, Stethoscope, UserCheck } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { KbdCmd } from "../../primitives/Kbd/Kbd";
import { AccountBadge, AccountMark, accountNeed, useAccount } from "../Account/Account";

/**
 * The title row's one menu: the account, Sleep, Settings and Doctor behind a single registry glyph, so the
 * row carries four things and not seven (the owner, 14 Oct 2026).
 *
 * **State that matters stays on the button.** Sleep on puts the moon on it, and nothing is signed in
 * puts an attention dot there, so neither needs the menu opened to be read. Off, the row shows no moon.
 *
 * The account entry is one row: Account and its mark, or Sign in, and never a login, since no profile is the active one. **It opens the account where the window keeps it**,
 * a page or a panel from the rail, and never holds the account itself: a sign-in in flight is the window's, so putting
 * the menu away loses nothing. While one waits on the person, or has just failed, the row and the button carry the
 * mark. Below the narrow bound the row's other controls are listed under these, which is the menu the row had before.
 */
export type TitleMenuProps = {
  /** Sleep mode's toggle. Absent draws no entry. */
  sleep?: { on: boolean; onToggle: () => void; calling?: boolean };
  /** Opens the account, a page or a panel. Absent draws no account entry. */
  onAccount?: () => void;
  /** Opens the Settings page. Absent draws no entry. */
  settings?: { onOpen: () => void; shortcut?: string };
  /** Opens the Doctor page. Absent draws no entry. */
  doctor?: { onOpen: () => void };
  /** The row's own controls, listed under the entries where the row is narrow. */
  narrow?: ReactNode;
};

export function TitleMenu({ sleep, settings, doctor, onAccount, narrow }: TitleMenuProps) {
  const account = useAccount();
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const signedOut = account !== undefined && account.profiles.length === 0;
  const need = account === undefined ? undefined : accountNeed(account);
  const sleeping = sleep?.on === true;

  const away = useRef(() => {});
  away.current = () => setOpen(false);

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

  if (account === undefined && sleep === undefined && settings === undefined && doctor === undefined && narrow === undefined) return null;

  const said = [
    "Menu",
    ...(sleeping ? ["sleep mode on"] : []),
    ...(signedOut ? ["signed out"] : []),
    ...(need === undefined ? [] : [`${need.said.slice(0, 1).toLowerCase()}${need.said.slice(1)}`]),
  ].join(", ");
  return (
    // Pressing anywhere in the menu is not the first use of Bridge that turns Sleep off.
    <div className="armada-title-bar__menu armada-popover" ref={root} data-keeps-sleep="">
      <span className="armada-popover__trigger">
        <span className="armada-title-menu__button" data-calling={sleep?.calling === true ? "" : undefined}>
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
          {account !== undefined && need !== undefined ? (
            <AccountBadge account={account} />
          ) : signedOut ? (
            <span className="armada-title-menu__attention" aria-hidden />
          ) : null}
        </span>
      </span>
      {open ? (
        <div className="armada-popover__panel armada-popover__panel--end armada-title-menu__panel" role="dialog" aria-label="Menu">
          <div className="armada-title-menu__entries">
              {account === undefined || onAccount === undefined ? null : (
                <button
                  type="button"
                  className="armada-title-menu__item"
                  aria-label={signedOut ? "Sign in" : "Account"}
                  onClick={() => {
                    onAccount();
                    away.current();
                  }}
                >
                  <span className="armada-title-menu__glyph">
                    <UserCheck size={16} strokeWidth={2} aria-hidden />
                  </span>
                  <span>{signedOut ? "Sign in" : "Account"}</span>
                  {need !== undefined ? (
                    <span className="armada-title-menu__need">
                      <AccountMark account={account} />
                    </span>
                  ) : signedOut ? (
                    <span className="armada-title-menu__attention armada-title-menu__attention--row" aria-hidden />
                  ) : null}
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
              {doctor === undefined ? null : (
                <button
                  type="button"
                  className="armada-title-menu__item"
                  aria-label="Doctor"
                  onClick={() => {
                    doctor.onOpen();
                    away.current();
                  }}
                >
                  <span className="armada-title-menu__glyph">
                    <Stethoscope size={16} strokeWidth={2} aria-hidden />
                  </span>
                  <span>Doctor</span>
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
        </div>
      ) : null}
    </div>
  );
}
