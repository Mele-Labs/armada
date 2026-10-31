import { Activity, BellRing, Check, FolderGit2, FolderPlus, Layers, MessageSquare, Plus, Search, Settings, type LucideIcon } from "lucide-react";
import { createContext, useContext, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { ArmadaLockupHorizontal, ArmadaMark } from "@armada/brand";
import { FLEET_DOT_TONE, FleetStarting, fleetSaid, type FleetState } from "../FleetPanel/FleetPanel";
import { Button } from "../../primitives/Button/Button";
import { DropdownMenu, filterEntries, type DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Input } from "../../primitives/Input/Input";
import { Popover } from "../../primitives/Popover/Popover";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { keyFor } from "../../actions";
import { Kbd, KbdBinding, KbdCmd } from "../../primitives/Kbd/Kbd";
import { useKeyBindings } from "../../keymap";
import { Inspect, InspectCard } from "../../primitives/Inspect/Inspect";
import { useShortcutReveal } from "../../shortcut-reveal";
import { usageLevel } from "../../usage";
import { TitleMenu } from "./TitleMenu";
import { useTeachMenu } from "../TeachPanel/TeachMenu";

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
 * and `seven_day` as `wk`; any other kind is not drawn. A list with no kind drawn is the rings loading.
 */
export type PlanUsage = { kind: string; percentUsed: number; resetsAt?: string; offer?: LimitOffer };

/**
 * What the owner is being offered on one limit: facts the card states under its own, acts it draws as
 * buttons, and **`attention`, the words of the bell on the meter** while something waits on him. Absent
 * draws the card as it was.
 */
/** One fact under a limit: its term, its value, and whether the value is a figure the system reported. */
export type LimitFact = readonly [term: string, value: ReactNode, mono?: boolean];

export type LimitOffer = {
  facts?: readonly LimitFact[];
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

type Drawn = { one: PlanUsage; limit: { short: string; name: string } };

/** One limit in the card: its name and how much is used on one line, then when it resets, any offer's facts and its acts. */
function LimitSection({ one, limit, close }: Drawn & { close: () => void }) {
  const percent = Math.round(one.percentUsed);
  const at = one.resetsAt === undefined ? undefined : new Date(one.resetsAt);
  const resets = at !== undefined && !Number.isNaN(at.getTime()) ? at : undefined;
  const rows: readonly LimitFact[] = [...(resets === undefined ? [] : [["Resets in", countdown(resets.getTime() - Date.now()), true] as const]), ...(one.offer?.facts ?? [])];
  return (
    <section className="armada-title-bar__usage-limit" data-level={usageLevel(percent)} aria-label={limit.name}>
      <p className="armada-title-bar__usage-head">
        {limit.name}
        <span className="armada-title-bar__usage-percent">{percent}%</span>
      </p>
      {rows.length === 0 ? null : (
        <dl className="armada-title-bar__usage-facts">
          {rows.map(([term, value, mono]) => (
            <div key={term}>
              <dt>{term}</dt>
              <dd data-mono={mono || undefined}>{value}</dd>
            </div>
          ))}
        </dl>
      )}
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
    </section>
  );
}

const NO_READING = "Analyzing Usage";

/** The ring glyph's place while Fleet has no reading: the same two tracks, an arc sweeping each. Not a control, as nothing opens. */
function PlanUsageLoading() {
  return (
    <span className="armada-title-bar__usage">
      <Tooltip label={NO_READING}>
        <span className="armada-title-bar__usage-loading" role="img" aria-label={NO_READING}>
          <svg width="22" height="22" viewBox="0 0 22 22" aria-hidden>
            {[9, 5].map((r) => (
              <g key={r}>
                <circle className="armada-title-bar__usage-ring-track" cx="11" cy="11" r={r} />
                <circle className="armada-title-bar__usage-ring-sweep" cx="11" cy="11" r={r} pathLength={100} strokeDasharray="25 75" />
              </g>
            ))}
          </svg>
        </span>
      </Tooltip>
    </span>
  );
}

/**
 * The plan's limits as one ring glyph, and the one card it opens: **hover or focus opens it, a press keeps it
 * open, and Esc or a press outside closes it**, as a metro stop's card does in the Bays view. The first limit is
 * the outer ring and the second the inner, each arc in its level's tone.
 */
function PlanUsageFigures({ usage }: { usage: readonly PlanUsage[] }) {
  const drawn: readonly Drawn[] = usage.flatMap((one) => {
    const limit = LIMITS[one.kind];
    return limit === undefined ? [] : [{ one, limit }];
  });
  if (drawn.length === 0) return <PlanUsageLoading />;
  const attention = drawn.find(({ one }) => one.offer?.attention !== undefined)?.one.offer?.attention;
  return (
    <span className="armada-title-bar__usage">
      <Inspect
        name="Plan limits"
        card={({ close }) => (
          <InspectCard kind={{ Glyph: Activity, label: "Plan" }} title="Plan limits" keys={[{ key: "Enter", verb: "Keep open" }]} width="wide">
            {drawn.map(({ one, limit }) => (
              <LimitSection key={one.kind} one={one} limit={limit} close={close} />
            ))}
          </InspectCard>
        )}
      >
        <button type="button" className="armada-title-bar__usage-meter" aria-label={drawn.map(({ one, limit }) => `${limit.name} ${Math.round(one.percentUsed)}%`).join(", ")}>
          <svg width="22" height="22" viewBox="0 0 22 22" aria-hidden>
            {drawn.map(({ one }, i) => {
              const percent = Math.min(100, Math.max(0, Math.round(one.percentUsed)));
              const r = 9 - i * 4;
              return (
                <g key={one.kind} data-level={usageLevel(percent)}>
                  <circle className="armada-title-bar__usage-ring-track" cx="11" cy="11" r={r} />
                  <circle className="armada-title-bar__usage-ring-fill" cx="11" cy="11" r={r} pathLength={100} strokeDasharray={`${percent} 100`} transform="rotate(-90 11 11)" />
                </g>
              );
            })}
          </svg>
          {attention === undefined ? null : (
            <Tooltip label={attention} beside>
              <span className="armada-title-bar__usage-attention" role="img" aria-label={attention}>
                <BellRing size={12} strokeWidth={2} aria-hidden />
              </span>
            </Tooltip>
          )}
        </button>
      </Inspect>
    </span>
  );
}

/** What the search trigger is called when it draws as an icon and its key, with its label hidden. */
const SEARCH_ONLY = "Search, or describe work to dispatch (⌘K)";

export type TitleBarProps = {
  /** Teach, from a right press on the bar or Shift+F10 in it. Absent draws no menu. */
  onTeach?: () => void;
  /**
   * The places the window has, drawn in the centre before the search trigger (`TabBar` is the Cockpit's).
   * **Absent draws the centre as it was.** Present, the trigger shrinks to an icon and its key, named by
   * its tooltip, and the centre stays on the row below the narrow breakpoint, where the picker's does not.
   */
  places?: ReactNode;
  /**
   * The brand at the row's start, after the traffic lights: a word or the mark. **It stands for the brand lockup
   * and mark at the end**, so the row draws neither there. Absent draws the lockup and the mark as they were.
   */
  wordmark?: ReactNode;
  /** The repository picker. Absent draws none — a bare bar until Bridge has one to offer. */
  repositoryPicker?: ReactNode;
  /**
   * The picked repository's name, as quiet text after the wordmark, for a surface that has no picker of its
   * own. Read-only: the pick is changed where the picker is. A long name is cut with an ellipsis and its
   * `title` holds the whole. Absent draws none. Hidden with the rest of the start below the narrow breakpoint.
   */
  repository?: string;
  /**
   * The picker's own entries, drawn flat in the narrow menu rather than as a
   * list opening inside it (the owner, 30 Sep 2026). Absent draws none there.
   */
  repositoryMenu?: {
    entries: DropdownMenuEntry[];
    onSelect: (id: string) => void;
    /** The Manifest button beside the picker in the full row. */
    manifest?: { label: string; icon: LucideIcon; disabled?: boolean; onOpen: () => void };
    /** A repository row's trailing mark and the act → runs on it, from the entry's id and label. Absent, or `undefined` for a row, draws none. */
    trailOf?: (id: string, label: string) => { node: ReactNode; open: () => void } | undefined;
    /** Opens a Workspace's page, from the gear on its heading row. Absent draws the heading with no gear. */
    onOpenWorkspace?: (id: string) => void;
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
    /** A call that lives in Helm is waiting: the button is tinted and pulses as a calling row does. */
    calling?: boolean;
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
    /** A decision Sleep made is waiting for him: the menu's button is tinted and pulses as a calling row does. */
    calling?: boolean;
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
   * The plan's limits, as two fill meters beside Sleep. **Absent draws nothing; an empty array draws the
   * rings loading**, Fleet's answer before any Session has reported. Not Fleet's dot: that stays liveness
   * and nothing else.
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

const PlacesRoom = createContext<number | undefined>(undefined);

/**
 * The width the places have in the row: what is left once the two sides and the search trigger are drawn. Read by
 * the places to choose what fits, so the strip stays on one line. `undefined` outside a title bar and before the
 * first measure.
 */
export function usePlacesRoom(): number | undefined {
  return useContext(PlacesRoom);
}

/**
 * The room is the row less its sides, the gaps and the trigger, the start taken **at its whole width**, the
 * repository's name untruncated: the places give up their labels before the name gives up any of itself.
 */
function useRoomForPlaces(on: boolean) {
  const bar = useRef<HTMLDivElement>(null);
  const start = useRef<HTMLDivElement>(null);
  const repository = useRef<HTMLSpanElement>(null);
  const center = useRef<HTMLDivElement>(null);
  const places = useRef<HTMLDivElement>(null);
  const search = useRef<HTMLButtonElement>(null);
  const end = useRef<HTMLDivElement>(null);
  const [room, setRoom] = useState<number>();
  useLayoutEffect(() => {
    const row = bar.current;
    if (!on || row === null) return;
    const measure = () => {
      const style = getComputedStyle(row);
      const inner = row.clientWidth - parseFloat(style.paddingInlineStart) - parseFloat(style.paddingInlineEnd);
      const gap = parseFloat(style.columnGap) || 0;
      const width = (part: HTMLElement | null) => part?.getBoundingClientRect().width ?? 0;
      const trigger = width(search.current);
      const between = center.current === null ? 0 : parseFloat(getComputedStyle(center.current).columnGap) || 0;
      // Drawn, the start clips and the name truncates; what either hides is added back.
      const name = repository.current;
      // `scrollWidth` is rounded, and a name a fraction too narrow draws its ellipsis: a pixel over.
      const whole = (start.current?.scrollWidth ?? 0) + (name === null ? 0 : name.scrollWidth - name.clientWidth) + 1;
      const searching = trigger > 0 ? trigger + between : 0;
      const content = width(places.current) + searching;
      const shown = start.current !== null && start.current.getClientRects().length > 0;
      const left = inner - 2 * gap - width(end.current) - content;
      row.style.setProperty("--armada-title-start-floor", `${Math.floor(shown ? Math.max(0, Math.min(whole, left)) : content)}px`);
      setRoom(Math.floor(inner - 2 * gap - whole - width(end.current) - searching));
    };
    measure();
    const watch = new ResizeObserver(measure);
    for (const part of [row, start.current, places.current, search.current, end.current]) if (part !== null) watch.observe(part);
    return () => watch.disconnect();
  }, [on]);
  return { bar, start, repository, center, places, search, end, room };
}

export function TitleBar({
  repositoryPicker,
  repository,
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
  onTeach,
}: TitleBarProps) {
  const revealing = useShortcutReveal();
  // The palette's key as this person bound it in Settings → Keyboard, redrawn when they rebind it.
  useKeyBindings();
  const searchKey = keyFor("command_palette");
  const parts = useRoomForPlaces(places !== undefined);
  const teach = useTeachMenu("Title bar", onTeach);
  return (
    <div ref={parts.bar} className="armada-title-bar" data-places={places === undefined ? undefined : ""} {...teach.on}>
      {teach.menu}
      <div ref={parts.start} className="armada-title-bar__start">
        {wordmark === undefined ? null : <div className="armada-title-bar__wordmark">{wordmark}</div>}
        {repository === undefined ? null : repositoryMenu === undefined ? (
          // `title`, not `Tooltip`: its bubble sits inside its anchor, and the anchor clips to cut the name.
          <span ref={parts.repository} className="armada-title-bar__repository" title={repository}>
            {repository}
          </span>
        ) : (
          // The name is the picker: it opens the repository list, and the Manifest's icon follows it.
          <span ref={parts.repository} className="armada-title-bar__repository" data-picker="">
            <span className="armada-title-bar__repository-chip">
            <span className="armada-title-bar__repository-glyph" aria-hidden>
              <FolderGit2 size={16} strokeWidth={2} />
            </span>
            <DropdownMenu
              plain
              portal
              align="start"
              triggerLabel={repository}
              triggerAriaLabel="Repository"
              triggerDetail=""
              searchLabel="Find a repository"
              panelClass="armada-title-bar__repository-list"
              entries={listed(repositoryMenu.entries, repositoryMenu.trailOf, repositoryMenu.onOpenWorkspace)}
              onSelect={repositoryMenu.onSelect}
            />
            </span>
            {repositoryMenu.manifest === undefined ? null : (
              <Tooltip label={repositoryMenu.manifest.label} asChild>
                <Button variant="ghost" size="sm" iconOnly aria-label={repositoryMenu.manifest.label} disabled={repositoryMenu.manifest.disabled} onClick={repositoryMenu.manifest.onOpen}>
                  <repositoryMenu.manifest.icon size={16} aria-hidden="true" />
                </Button>
              </Tooltip>
            )}
          </span>
        )}
        {repositoryPicker === undefined ? null : (
          <div className="armada-title-bar__picker">{repositoryPicker}</div>
        )}
      </div>

      <div ref={parts.center} className="armada-title-bar__center">
        {places === undefined ? null : (
          <div ref={parts.places} className="armada-title-bar__places">
            <PlacesRoom.Provider value={parts.room}>{places}</PlacesRoom.Provider>
          </div>
        )}
        {onSearch === undefined ? null : (
          <button
            ref={parts.search}
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

      <div ref={parts.end} className="armada-title-bar__end">
        {planUsage === undefined ? null : <PlanUsageFigures usage={planUsage} />}

        {helm === undefined ? null : (
          <button
            type="button"
            className="armada-title-bar__helm"
            data-calling={helm.calling === true ? "" : undefined}
            onClick={helm.onOpen}
            title={helm.binding === undefined ? "Open Helm" : `Open Helm — ${helm.binding}`}
            aria-label="Helm"
          >
            <MessageSquare size={16} strokeWidth={2} aria-hidden />
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

/** The shell's ids for the picker's acts and for All: `repository-entries.ts`, which this package cannot import. */
const REPOSITORY_ACTS: Readonly<Record<string, LucideIcon>> = { "new-project": FolderPlus, "add-repository": Plus, "new-workspace": Plus };

/** A Workspace's heading row id starts with this, and then the Workspace's id. */
const WORKSPACE = "workspace:";

/**
 * The wide picker's rows told apart: a glyph before each repository (Layers for All), a quiet tag on one that is not set
 * up in place of its heading, and the acts below the rule as verbs with their own glyphs.
 */
function listed(entries: DropdownMenuEntry[], trailOf: NonNullable<TitleBarProps["repositoryMenu"]>["trailOf"], onOpenWorkspace?: (id: string) => void): DropdownMenuEntry[] {
  let loose = false;
  return entries.flatMap((entry): DropdownMenuEntry[] => {
    if (entry.kind === "separator") loose = false;
    if (entry.kind === "label") {
      loose = entry.id === "not-set-up";
      return loose ? [] : [entry];
    }
    if (entry.kind !== "item") return [entry];
    const act = REPOSITORY_ACTS[entry.id];
    if (act !== undefined) return [{ ...entry, icon: act, act: true }];
    // A Workspace is a heading over its repositories: it is not chosen, and its gear opens its page.
    if (entry.id.startsWith(WORKSPACE)) {
      const id = entry.id.slice(WORKSPACE.length);
      const gear = (
        <Tooltip label={`Open ${entry.label}`}>
          <span role="button" tabIndex={-1} className="armada-title-bar__workspace-gear" aria-label={`Open ${entry.label}`}>
            <Settings size={16} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      );
      return [{ ...entry, icon: Layers, disabled: true, ...(onOpenWorkspace === undefined ? {} : { trail: gear, onRight: () => onOpenWorkspace(id) }) }];
    }
    const trail = entry.id === "" ? undefined : trailOf?.(entry.id, entry.label);
    return [{ ...entry, icon: entry.id === "" ? Layers : FolderGit2, ...(loose ? { detail: "Not set up" } : {}), ...(trail === undefined ? {} : { trail: trail.node, onRight: trail.open }) }];
  });
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
