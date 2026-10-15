import { BellRing, Check, MessageSquare, Plus, Search, type LucideIcon } from "lucide-react";
import { useState, type CSSProperties, type ReactNode } from "react";
import { ArmadaLockupHorizontal, ArmadaMark } from "@armada/brand";
import { FLEET_DOT_TONE, FleetStarting, fleetSaid, type FleetState } from "../FleetPanel/FleetPanel";
import { Button } from "../../primitives/Button/Button";
import { filterEntries, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Input } from "../../primitives/Input/Input";
import { Popover } from "../../primitives/Popover/Popover";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { keyFor } from "../../actions";
import { Kbd, KbdBinding, KbdCmd } from "../../primitives/Kbd/Kbd";
import { useKeyBindings } from "../../keymap";
import { StopCard, useCardOpen, type StopCardRow } from "../StopCard/StopCard";
import { useShortcutReveal } from "../../shortcut-reveal";
import { usageLevel } from "../../usage";
import { TitleMenu } from "./TitleMenu";

/**
 * The title row — what used to be macOS's grey bar saying only "Armada".
 * `titleBarStyle: 'hiddenInset'` insets the traffic lights over this bar
 * rather than over the sidebar; the whole row is a drag region, and every
 * control in it opts out with its own no-drag rule. #1087.
 *
 * **Three regions, not one flex row with a trailing spacer.** Left carries the
 * repository picker, centered carries the one bar, right carries Helm's reopen
 * control and the logo — `TitleBar.css`'s grid keeps the one bar on the bar's
 * true midpoint however wide the two sides measure. The one bar also dispatches.
 *
 * **A slot, not a decision.** `repositoryPicker` arrives built: the Select and
 * its options are Bridge's own reading of what Fleet serves, and this row only
 * has room for it, the way `TheShell`'s `railHeader` used to.
 */
/**
 * One limit of the plan, as the mod's `session.measure` carries it. `five_hour` draws as `5h`
 * and `seven_day` as `wk`; any other kind is not drawn.
 */
export type PlanUsage = { kind: string; percentUsed: number; resetsAt?: string; offer?: LimitOffer };

/**
 * What the owner is being offered on one limit: facts the card states under its own, acts it draws as
 * buttons, and **`attention`, the words of the bell on the meter** while something waits on him. Absent
 * draws the card as it was.
 */
export type LimitOffer = {
  facts?: readonly StopCardRow[];
  acts?: readonly { label: string; onPress: () => void; /** Keeps the card open after, for an act whose result the card states. Closes it otherwise. */ stays?: boolean; primary?: boolean }[];
  attention?: string;
};

const LIMITS: Record<string, { short: string; name: string } | undefined> = {
  five_hour: { short: "5h", name: "5-hour limit" },
  seven_day: { short: "wk", name: "Weekly limit" },
};

/** "2h 14m", "2d 17h", "9m": the two largest units left. */
function countdown(ms: number): string {
  const minutes = Math.max(0, Math.floor(ms / 60_000));
  const days = Math.floor(minutes / 1_440);
  const hours = Math.floor((minutes % 1_440) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  return `${minutes}m`;
}

/**
 * One limit as a fill meter, and the card it opens: **hover or focus opens it, a press keeps it open, and
 * Esc or a press outside closes it**, as a metro stop's card does in the Bays view. No acts.
 */
function LimitMeter({ one, limit }: { one: PlanUsage; limit: { short: string; name: string } }) {
  const { open, pinned, toggle, close, bind } = useCardOpen();
  const percent = Math.round(one.percentUsed);
  const level = usageLevel(percent);
  const at = one.resetsAt === undefined ? undefined : new Date(one.resetsAt);
  const resets = at !== undefined && !Number.isNaN(at.getTime()) ? at : undefined;
  return (
    <span className="armada-title-bar__usage-limit" data-level={level} {...bind}>
      <button
        type="button"
        className="armada-title-bar__usage-meter"
        aria-label={`${limit.name} ${percent}%`}
        aria-expanded={open}
        style={{ "--fill": `${Math.min(100, Math.max(0, percent))}%` } as CSSProperties}
        onClick={toggle}
      >
        <span className="armada-title-bar__usage-label">{limit.short}</span>
        <span className="armada-title-bar__usage-track" />
        {one.offer?.attention === undefined ? null : (
          <Tooltip label={one.offer.attention} beside>
            <span className="armada-title-bar__usage-attention" role="img" aria-label={one.offer.attention}>
              <BellRing size={12} strokeWidth={2} aria-hidden />
            </span>
          </Tooltip>
        )}
      </button>
      {open ? (
        <StopCard
          className="armada-title-bar__usage-card"
          role="dialog"
          aria-label={limit.name}
          data-pinned={pinned || undefined}
          pill={`${percent}% used`}
          where={limit.short}
          title={limit.name}
          rows={[...(resets === undefined ? [] : [["Resets in", countdown(resets.getTime() - Date.now()), true] as const]), ...(one.offer?.facts ?? [])]}
        >
          {one.offer?.acts === undefined || one.offer.acts.length === 0 ? null : (
            <span className="armada-title-bar__usage-acts">
              {one.offer.acts.map((act) => (
                <Button
                  key={act.label}
                  variant={act.primary === true ? "primary" : "secondary"}
                  size="sm"
                  onClick={() => {
                    act.onPress();
                    if (act.stays !== true) close();
                  }}
                >
                  {act.label}
                </Button>
              ))}
            </span>
          )}
        </StopCard>
      ) : null}
    </span>
  );
}

function PlanUsageFigures({ usage }: { usage: readonly PlanUsage[] }) {
  const drawn = usage.flatMap((one) => {
    const limit = LIMITS[one.kind];
    return limit === undefined ? [] : [{ one, limit }];
  });
  if (drawn.length === 0) return null;
  return (
    <div className="armada-title-bar__usage">
      {drawn.map(({ one, limit }) => (
        <LimitMeter key={one.kind} one={one} limit={limit} />
      ))}
    </div>
  );
}

/** What the search trigger is called when it draws as an icon and its key, with its label hidden. */
const SEARCH_ONLY = "Search, or describe work to dispatch (⌘K)";

export type TitleBarProps = {
  /**
   * The places the window has, drawn in the centre before the search trigger (`TabBar` is the Cockpit's).
   * **Absent draws the centre as it was.** Present, the trigger shrinks to an icon and its key, named by
   * its tooltip, and the centre stays on the row below the narrow breakpoint, where the picker's does not.
   */
  places?: ReactNode;
  /**
   * A word at the row's start, after the traffic lights. **It stands for the brand lockup and mark**, so
   * the row draws neither beside it. Absent draws the lockup and the mark as they were.
   */
  wordmark?: ReactNode;
  /** The repository picker. Absent draws none — a bare bar until Bridge has one to offer. */
  repositoryPicker?: ReactNode;
  /**
   * The picker's own entries, drawn flat in the narrow menu rather than as a
   * list opening inside it (the owner, 30 Sep 2026). Absent draws none there.
   */
  repositoryMenu?: {
    entries: DropdownMenuEntry[];
    onSelect: (id: string) => void;
    /** The Manifest button beside the picker in the full row. */
    manifest?: { label: string; icon: LucideIcon; disabled?: boolean; onOpen: () => void };
  };
  /** Opens the command palette. The same surface ⌘K opens; this is the other way in. */
  onSearch?: () => void;
  /**
   * Opens the composer, from the narrow menu. The full row has no Dispatch:
   * its one bar dispatches what is typed in it.
   */
  onDispatch?: () => void;
  /** Disabled while nothing is connected to dispatch into. */
  dispatchDisabled?: boolean;
  /** Helm's reopen control. Absent while the dock is already open beside the content. */
  helm?: {
    /** Waiting on a person, across every repository. Zero draws no count. */
    questions: number;
    /** ⌘J, beside the label. */
    binding?: string;
    onOpen: () => void;
  };
  /**
   * Sleep mode, an entry in the row's menu: the machine keeps working until it
   * is blocked on the owner. While on, the menu's button carries the moon.
   * **Absent draws no entry**, so a Bridge with no sleep to offer shows none.
   */
  sleep?: {
    on: boolean;
    onToggle: () => void;
  };
  /**
   * What the row's one menu opens besides the account and Sleep. **Absent draws no entry**: a Bridge with
   * no Settings page to offer shows none.
   */
  menu?: {
    /** Opens the account, a page or a panel, from its row. Absent draws no account row. */
    account?: { onOpen: () => void };
    settings?: { onOpen: () => void; /** `⌘,` or whichever key the person has it on. */ shortcut?: string };
    /** Opens the Doctor page, from its row. Absent draws no Doctor row. */
    doctor?: { onOpen: () => void };
  };
  /**
   * The plan's limits, as two fill meters beside Sleep. **Absent, or with no limit this row
   * knows, draws nothing.** Not Fleet's dot: that stays liveness and nothing else.
   */
  planUsage?: readonly PlanUsage[];
  /**
   * Fleet's liveness, as one dot on the bar's trailing edge. **Drawn at every
   * width**, settled by the owner on 18 Sep 2026. #1438 drew it only while the
   * left column was absent; both that condition and removing the dot outright
   * were shown to him and rejected — it is chrome, and chrome that appears and
   * disappears with a resize is the thing worth avoiding, not the second dot.
   *
   * Required for that reason: a title row Bridge draws without Fleet's state
   * is one the contract does not describe.
   *
   * **Liveness and nothing else.** pid, port, protocol and uptime stay in the
   * Fleet panel; a dot cannot carry a figure.
   */
  fleet: {
    state: FleetState;
    /** The panel's own word — "Running", "Not running". Read for the name and the tooltip. */
    label: string;
    /**
     * What the dot opens, in a popover on the bar: Fleet's panel, for a window that draws no rail to
     * hold it. **Present makes the dot a button**; absent leaves it the graphic it was, because the panel
     * is already on screen in the left column.
     */
    panel?: ReactNode;
  };
};

export function TitleBar({
  repositoryPicker,
  repositoryMenu,
  onSearch,
  onDispatch,
  dispatchDisabled = false,
  helm,
  sleep,
  menu,
  planUsage,
  fleet,
  places,
  wordmark,
}: TitleBarProps) {
  const revealing = useShortcutReveal();
  // The palette's key as this person bound it in Settings → Keyboard, redrawn when they rebind it.
  useKeyBindings();
  const searchKey = keyFor("command_palette");
  return (
    <div className="armada-title-bar" data-places={places === undefined ? undefined : ""}>
      <div className="armada-title-bar__start">
        {wordmark === undefined ? null : <div className="armada-title-bar__wordmark">{wordmark}</div>}
        {repositoryPicker === undefined ? null : (
          <div className="armada-title-bar__picker">{repositoryPicker}</div>
        )}
      </div>

      <div className="armada-title-bar__center">
        {places === undefined ? null : <div className="armada-title-bar__places">{places}</div>}
        {onSearch === undefined ? null : (
          <button
            type="button"
            className="armada-title-bar__search"
            onClick={onSearch}
            {...(places === undefined ? {} : { title: SEARCH_ONLY, "aria-label": SEARCH_ONLY })}
          >
            <Search size={16} strokeWidth={2} aria-hidden />
            <span className="armada-title-bar__search-label">Search, or describe work to dispatch</span>
            {searchKey === "" ? null : <KbdBinding binding={searchKey} />}
          </button>
        )}
      </div>

      <div className="armada-title-bar__end">
        {planUsage === undefined ? null : <PlanUsageFigures usage={planUsage} />}

        {helm === undefined ? null : (
          <button
            type="button"
            className="armada-title-bar__helm"
            onClick={helm.onOpen}
            title={helm.binding === undefined ? "Open Helm" : `Open Helm — ${helm.binding}`}
          >
            <MessageSquare size={16} strokeWidth={2} aria-hidden />
            <span>Helm</span>
            {helm.questions > 0 ? (
              <span className="armada-title-bar__helm-count">{helm.questions}</span>
            ) : null}
            {/* Mounted only while Cmd is held, so the button shrinks to fit
                "Helm" alone at rest rather than always reserving the badge's
                width — the logo beside it shifts a few px when this mounts. */}
            {helm.binding !== undefined && revealing ? (
              <span className="armada-title-bar__helmkbd">
                <KbdCmd shortcut={helm.binding} />
              </span>
            ) : null}
          </button>
        )}

        {/* Both drawn; the stylesheet shows one by the bar's width. A wordmark stands for them. */}
        {wordmark !== undefined ? null : (
          <>
            <span className="armada-title-bar__lockup">
              <ArmadaLockupHorizontal height={20} title="Armada" />
            </span>
            <span className="armada-title-bar__mark">
              <ArmadaMark size={20} title="Armada" />
            </span>
          </>
        )}

        {/* Last, on the bar's own trailing edge — where the owner put it on
            17 Sep 2026. The dot is `--dot`, the same 6px the panel and the
            48px rail draw, so the three readings of one fact are one size as
            well as one colour. */}
        {fleet.panel === undefined ? (
          <span
            className="armada-title-bar__fleet"
            // A graphic with a name, `StepBar`'s own pattern, rather than a live
            // region: it would announce a state nobody asked about, and now that
            // it never mounts on a resize there is nothing new to announce
            // anyway — the label simply changes with Fleet.
            role="img"
            aria-label={fleetSaid(fleet.label)}
            // `title`, not the `Tooltip` primitive: it is what this row already
            // gives its Helm button, and a tooltip's own surface at `--z-tooltip`
            // has nothing to anchor to on a 6px dot. The no-drag rule in
            // `TitleBar.css` is what lets either open at all — the whole row is
            // a drag region and a drag region swallows the pointer.
            title={fleetSaid(fleet.label)}
          >
            <FleetDot state={fleet.state} />
          </span>
        ) : (
          <span className="armada-title-bar__fleet">
            <Popover
              align="end"
              label="Fleet"
              trigger={
                <button type="button" className="armada-title-bar__fleet-button" aria-label={fleetSaid(fleet.label)} title={fleetSaid(fleet.label)}>
                  <FleetDot state={fleet.state} />
                </button>
              }
            >
              {fleet.panel}
            </Popover>
          </span>
        )}

        {/* The row's one menu, at every width: the account, Sleep, Settings and Doctor, and under them, where the
            row is narrow, everything the row above carries. */}
        <TitleMenu
          {...(sleep === undefined ? {} : { sleep })}
          {...(menu?.settings === undefined ? {} : { settings: menu.settings })}
          {...(menu?.doctor === undefined ? {} : { doctor: menu.doctor })}
          {...(menu?.account === undefined ? {} : { onAccount: menu.account.onOpen })}
          narrow={
            <>
              {repositoryMenu === undefined ? null : <RepositoryRows {...repositoryMenu} />}
              {onSearch === undefined ? null : (
                <button type="button" className="armada-title-bar__menu-item" onClick={onSearch}>
                  <Search size={16} strokeWidth={2} aria-hidden />
                  <span>Search</span>
                  {searchKey === "" ? null : <KbdBinding binding={searchKey} />}
                </button>
              )}
              {onDispatch === undefined ? null : (
                <button type="button" className="armada-title-bar__menu-item" onClick={onDispatch} disabled={dispatchDisabled}>
                  <Plus size={16} strokeWidth={2} aria-hidden />
                  <span>Dispatch</span>
                  <Kbd aria-hidden>{keyFor("new_job")}</Kbd>
                </button>
              )}
              {helm === undefined ? null : (
                <button type="button" className="armada-title-bar__menu-item" onClick={helm.onOpen}>
                  <MessageSquare size={16} strokeWidth={2} aria-hidden />
                  <span>Helm</span>
                  {helm.questions > 0 ? <span className="armada-title-bar__helm-count">{helm.questions}</span> : null}
                  {helm.binding === undefined ? null : <KbdCmd shortcut={helm.binding} />}
                </button>
              )}
            </>
          }
        />
      </div>
    </div>
  );
}

function FleetDot({ state }: { state: FleetState }) {
  return state === "starting" ? (
    <FleetStarting className="armada-fleet-starting" />
  ) : (
    <span className="armada-title-bar__fleet-dot" data-tone={FLEET_DOT_TONE[state]} aria-hidden />
  );
}

/** The picker's list, flat: the same entries its dropdown draws, one row each. */
function RepositoryRows({ entries, onSelect, manifest }: NonNullable<TitleBarProps["repositoryMenu"]>) {
  // The wide picker's filter field, in the narrow menu too: a list of twenty is a scroll here as well.
  const [query, setQuery] = useState("");
  const shown = filterEntries(entries, query);
  return (
    <div className="armada-title-bar__menu-repositories" role="group" aria-label="Repositories">
      <div className="armada-title-bar__menu-search">
        <Input aria-label="Find a repository" role="searchbox" autoComplete="off" value={query} onChange={(event) => setQuery(event.target.value)} />
      </div>
      {query.trim() !== "" && shown.length === 0 ? (
        <p className="armada-title-bar__menu-empty" role="status">{`Nothing matches “${query.trim()}”.`}</p>
      ) : null}
      {shown.map((entry) =>
        entry.kind === "separator" ? (
          <div key={entry.id} className="armada-title-bar__menu-rule" aria-hidden />
        ) : entry.kind === "label" ? (
          <div key={entry.id} className="armada-title-bar__menu-label">
            {entry.label}
          </div>
        ) : (
          <button
            key={entry.id}
            type="button"
            className="armada-title-bar__menu-item"
            aria-pressed={entry.selected === true}
            onClick={() => onSelect(entry.id)}
          >
            <span className="armada-title-bar__menu-check" aria-hidden>
              {entry.selected === true ? <Check size={16} strokeWidth={2} /> : null}
            </span>
            <span>{entry.label}</span>
          </button>
        ),
      )}
      {manifest === undefined ? null : (
        <button
          type="button"
          className="armada-title-bar__menu-item"
          disabled={manifest.disabled}
          onClick={manifest.onOpen}
        >
          <manifest.icon size={16} strokeWidth={2} aria-hidden />
          <span>{manifest.label}</span>
        </button>
      )}
    </div>
  );
}
