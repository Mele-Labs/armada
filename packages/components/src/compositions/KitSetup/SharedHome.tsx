import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import { ArrowDownToLine, ArrowUpFromLine, Check, CircleDashed, House, Key, Laptop, RotateCw, Split } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { ModKind } from "../ModRow/ModRow";
import { HomePicker } from "./HomeChoice";

/**
 * Kit against the shared home: a private repository on the person's own GitHub
 * account that carries their Kit, workflows, allowlist, a repository's memory
 * and the global instructions to every machine they sign in on.
 *
 * **Each row says how it stands against the home in a glyph and nothing else.**
 * A row that differs is never merged: it opens both versions side by side and
 * the person picks a side. Nothing is drawn where no home is provided, so a
 * Kit with no home reads exactly as it did.
 */
export type SyncState = "same" | "differs" | "here" | "home";

export type SharedRow = {
  /** The tile it stands in, by `KitSetupKindWord`. */
  kind: string;
  name: string;
  state: SyncState;
  /** What a row that is not on this machine says for itself. */
  says?: string;
  /** Where a row that is not on this machine would come from. */
  source?: string;
  /** A server whose key is not on this machine. Name and host are all that travelled. */
  keyHere?: boolean;
  /** Mods only: what a mod changes, so a mod only the home has is listed under its own kind. */
  modKind?: ModKind;
  /** The two versions of a row that differs. */
  here?: string;
  there?: string;
};

export type SharedHomeRead = {
  /** The GitHub login the home belongs to, or the organisation it is made under. */
  account: string;
  /** The signed-in login itself, where `account` may be an organisation. Absent is that `account` is the login. */
  login?: string;
  /** The home's repository name. */
  repository: string;
  rows: SharedRow[];
  syncing: boolean;
  /** Whether the home exists. Absent is that it does. */
  made?: boolean;
  /** The organisations the account belongs to. Where the home is not made and this is given, the chip opens a picker for its owner and name. */
  owners?: readonly string[];
};

export type Side = "here" | "there";

export type SharedHomeActs = {
  /** Keep one side of a row that differs. */
  choose: (row: SharedRow, side: Side) => void;
  /** Put a row that is only on this machine into the home. */
  add: (row: SharedRow) => void;
  /** Bring a row that is only in the home to this machine. */
  bring: (row: SharedRow) => void;
  sync: () => void;
  /** Choose where the home is made: the owner and the repository name. Absent keeps the choice on this window alone. */
  setHome?: (owner: string, name: string) => void;
};

export type SharedHome = SharedHomeRead & SharedHomeActs;

const Held = createContext<SharedHome | undefined>(undefined);

export function SharedHomeProvider({ value, children }: { value: SharedHome | undefined; children: ReactNode }) {
  return <Held.Provider value={value}>{children}</Held.Provider>;
}

/** The home this window syncs against. `undefined` where there is none. */
export function useSharedHome(): SharedHome | undefined {
  return useContext(Held);
}

const STATE = {
  same: { Glyph: Check, said: "Same as the home" },
  differs: { Glyph: Split, said: "Differs from the home" },
  here: { Glyph: Laptop, said: "Only on this machine" },
  home: { Glyph: House, said: "Only in the home" },
} as const;

/** What a row's tooltip says of how it stands. */
export const syncSaid = (state: SyncState): string => STATE[state].said;

/**
 * A row's marks. One glyph for how it stands, and a key beside it for a server
 * whose key this machine has not got. A row that differs is the one mark that
 * is a button, because it opens the two versions.
 */
export function SyncMarks({ row, onCompare }: { row: SharedRow; onCompare: (row: SharedRow) => void }) {
  const { Glyph, said } = STATE[row.state];
  const glyph = <Glyph size={16} strokeWidth={2} aria-hidden="true" />;
  return (
    <span className="armada-sync-marks">
      {row.keyHere === true ? (
        <Tooltip beside label="Needs its key on this machine">
          <span className="armada-sync-mark" data-sync="key" role="img" aria-label="Needs its key on this machine">
            <Key size={16} strokeWidth={2} aria-hidden="true" />
          </span>
        </Tooltip>
      ) : null}
      <Tooltip beside label={said}>
        {row.state === "differs" ? (
          <button type="button" className="armada-sync-mark" data-sync="differs" aria-label={`${said}, ${row.name}`} onClick={() => onCompare(row)}>
            {glyph}
          </button>
        ) : (
          <span className="armada-sync-mark" data-sync={row.state} role="img" aria-label={said}>
            {glyph}
          </span>
        )}
      </Tooltip>
    </span>
  );
}

/**
 * The act a row offers, as an icon button with its tooltip and no text: up out of this machine
 * where the row is only here, down onto it where the row is only in the home.
 */
export function SyncAct({ row, acts }: { row: SharedRow; acts: SharedHomeActs }) {
  if (row.state !== "here" && row.state !== "home") return null;
  const up = row.state === "here";
  const said = up ? "Add to the home" : "Add to this machine";
  const Glyph = up ? ArrowUpFromLine : ArrowDownToLine;
  return (
    <Tooltip beside label={said}>
      <Button variant="ghost" size="sm" iconOnly aria-label={`${said} ${row.name}`} onClick={() => (up ? acts.add(row) : acts.bring(row))}>
        <Glyph size={16} strokeWidth={2} aria-hidden="true" />
      </Button>
    </Tooltip>
  );
}

/** The home's repository, `account/repository`. `made` false marks one no kind has synced to yet. */
export function SharedHomeChip({ account, repository, made = true }: { account: string; repository: string; made?: boolean }) {
  return (
    <Tooltip label={`${made ? "Shared home" : "Shared home, not made yet"}: ${account}/${repository}`}>
      <span className="armada-shared-chip" data-made={made ? undefined : "no"}>
        {made ? <House size={12} strokeWidth={2} aria-hidden="true" /> : <CircleDashed size={12} strokeWidth={2} aria-hidden="true" />}
        <span>
          {account}
          <span className="armada-shared-chip__repo">/{repository}</span>
        </span>
      </span>
    </Tooltip>
  );
}

/** The account the home belongs to, and Sync. */
export function SharedHomeBar({ home }: { home: SharedHome }) {
  return (
    <div className="armada-shared-bar">
      {home.made === false && home.owners !== undefined ? (
        <HomePicker
          login={home.login ?? home.account}
          owners={home.owners}
          {...(home.setHome === undefined ? {} : { chosen: { owner: home.account, name: home.repository }, onChoose: home.setHome })}
        />
      ) : (
        <SharedHomeChip account={home.account} repository={home.repository} made={home.made !== false} />
      )}
      <Button variant="ghost" size="sm" pending={home.syncing} onClick={home.sync}>
        <RotateCw size={12} strokeWidth={2} aria-hidden="true" className="armada-shared-bar__turn" data-turning={home.syncing ? "" : undefined} />
        Sync
      </Button>
    </div>
  );
}

/** The two versions of a row that differs, side by side, and the two acts that settle it. */
export function SyncSheet({
  row,
  kindLabel,
  acts,
  onClose,
}: {
  row: SharedRow;
  kindLabel: string;
  acts: SharedHomeActs;
  onClose: () => void;
}) {
  const here = (row.here ?? "").split("\n");
  const there = (row.there ?? "").split("\n");
  const choose = (side: Side) => {
    acts.choose(row, side);
    onClose();
  };
  return (
    <Sheet
      kind="shared-home-row"
      open
      floating
      size="widest"
      title={row.name}
      subtitle={kindLabel}
      closeLabel="Close panel"
      closeBinding="Esc"
      footer={
        <>
          <Button variant="secondary" size="sm" ground="sunken" onClick={() => choose("here")}>
            Use this machine&rsquo;s
          </Button>
          <Button variant="secondary" size="sm" ground="sunken" onClick={() => choose("there")}>
            Use the home&rsquo;s
          </Button>
        </>
      }
      onClose={onClose}
    >
      <div className="armada-sync-sides">
        <Version Glyph={Laptop} name="This machine" lines={here} other={there} />
        <Version Glyph={House} name="The home" lines={there} other={here} />
      </div>
    </Sheet>
  );
}

/** One side: its band, then its lines, each one that the other side lacks lit. */
function Version({ Glyph, name, lines, other }: { Glyph: typeof House; name: string; lines: string[]; other: string[] }) {
  const theirs = new Set(other);
  return (
    <section className="armada-sync-side" aria-label={name}>
      <header className="armada-sync-side__band">
        <Glyph size={12} strokeWidth={2} aria-hidden="true" />
        {name}
      </header>
      <ol className="armada-sync-side__lines">
        {lines.map((line, at) => (
          <li key={at} className="armada-sync-side__line" data-differs={theirs.has(line) ? undefined : ""}>
            {line}
          </li>
        ))}
      </ol>
    </section>
  );
}
