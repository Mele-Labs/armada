import { Check, Menu, MessageSquare, Plus, Search, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { ArmadaLockupHorizontal, ArmadaMark } from "@armada/brand";
import { FLEET_DOT_TONE, FleetStarting, fleetSaid, type FleetState } from "../FleetPanel/FleetPanel";
import { Button } from "../../primitives/Button/Button";
import type { DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { actionOf } from "../../actions";
import { Kbd, KbdCmd } from "../../primitives/Kbd/Kbd";
import { Popover } from "../../primitives/Popover/Popover";
import { useShortcutReveal } from "../../shortcut-reveal";

/**
 * The title row — what used to be macOS's grey bar saying only "Armada".
 * `titleBarStyle: 'hiddenInset'` insets the traffic lights over this bar
 * rather than over the sidebar; the whole row is a drag region, and every
 * control in it opts out with its own no-drag rule. #1087.
 *
 * **Three regions, not one flex row with a trailing spacer.** Left carries the
 * repository picker, centered carries search and Dispatch as a group, right
 * carries Helm's reopen control and the logo — `TitleBar.css`'s grid keeps the
 * center group on the bar's true midpoint however wide the two sides measure.
 *
 * **A slot, not a decision.** `repositoryPicker` arrives built: the Select and
 * its options are Bridge's own reading of what Fleet serves, and this row only
 * has room for it, the way `TheShell`'s `railHeader` used to.
 */
export type TitleBarProps = {
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
   * Opens the composer.
   *
   * Dispatch here is a plain button, not a split one. #1156: a
   * `SplitButton` with nothing behind its caret still drew a caret, so a
   * person saw a menu that was not there.
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
   * Fleet panel; a dot cannot carry a figure and this row is not where one is
   * read.
   */
  fleet: {
    state: FleetState;
    /** The panel's own word — "Running", "Not running". Read for the name and the tooltip. */
    label: string;
  };
};

export function TitleBar({
  repositoryPicker,
  repositoryMenu,
  onSearch,
  onDispatch,
  dispatchDisabled = false,
  helm,
  fleet,
}: TitleBarProps) {
  const revealing = useShortcutReveal();
  return (
    <div className="armada-title-bar">
      <div className="armada-title-bar__start">
        {repositoryPicker === undefined ? null : (
          <div className="armada-title-bar__picker">{repositoryPicker}</div>
        )}
      </div>

      <div className="armada-title-bar__center">
        {onSearch === undefined ? null : (
          <button type="button" className="armada-title-bar__search" onClick={onSearch}>
            <Search size={16} strokeWidth={2} aria-hidden />
            <span className="armada-title-bar__search-label">Search jobs, commands, settings…</span>
            <KbdCmd shortcut="⌘K" />
          </button>
        )}

        {onDispatch === undefined ? null : (
          // Wrapped, not styled directly: `Button` takes no `className` to
          // carry a title-bar-only no-drag hook on, same reasoning as the
          // picker below. #1156.
          <div className="armada-title-bar__dispatch">
            <Button variant="tonal" size="sm" onClick={onDispatch} disabled={dispatchDisabled}>
              <Plus size={16} strokeWidth={2} aria-hidden />
              Dispatch
            </Button>
          </div>
        )}
      </div>

      <div className="armada-title-bar__end">
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

        {/* Both drawn; the stylesheet shows one by the bar's width. */}
        <span className="armada-title-bar__lockup">
          <ArmadaLockupHorizontal height={20} title="Armada" />
        </span>
        <span className="armada-title-bar__mark">
          <ArmadaMark size={20} title="Armada" />
        </span>

        {/* Last, on the bar's own trailing edge — where the owner put it on
            17 Sep 2026. The dot is `--dot`, the same 6px the panel and the
            48px rail draw, so the three readings of one fact are one size as
            well as one colour. */}
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
          {fleet.state === "starting" ? (
            <FleetStarting className="armada-fleet-starting" />
          ) : (
            <span className="armada-title-bar__fleet-dot" data-tone={FLEET_DOT_TONE[fleet.state]} aria-hidden />
          )}
        </span>

        {/* The narrow bar's one control: everything the row above carries, in
            a panel. Drawn always and shown by the stylesheet, like the mark. */}
        <div className="armada-title-bar__menu">
          <Popover
            align="end"
            label="Menu"
            trigger={
              <Button variant="ghost" size="sm" iconOnly aria-label="Menu" title="Menu">
                <Menu size={16} strokeWidth={2} aria-hidden />
              </Button>
            }
          >
            {(close) => (
              <div
                className="armada-title-bar__menu-panel"
                // Any act closes the panel.
                onClick={(event) => {
                  if ((event.target as Element).closest("button:not(:disabled)") !== null) close();
                }}
              >
                {repositoryMenu === undefined ? null : <RepositoryRows {...repositoryMenu} />}
                {onSearch === undefined ? null : (
                  <button type="button" className="armada-title-bar__menu-item" onClick={onSearch}>
                    <Search size={16} strokeWidth={2} aria-hidden />
                    <span>Search</span>
                    <KbdCmd shortcut="⌘K" />
                  </button>
                )}
                {onDispatch === undefined ? null : (
                  <button
                    type="button"
                    className="armada-title-bar__menu-item"
                    onClick={onDispatch}
                    disabled={dispatchDisabled}
                  >
                    <Plus size={16} strokeWidth={2} aria-hidden />
                    <span>Dispatch</span>
                    <Kbd aria-hidden>{actionOf("new_job").shortcut}</Kbd>
                  </button>
                )}
                {helm === undefined ? null : (
                  <button type="button" className="armada-title-bar__menu-item" onClick={helm.onOpen}>
                    <MessageSquare size={16} strokeWidth={2} aria-hidden />
                    <span>Helm</span>
                    {helm.questions > 0 ? (
                      <span className="armada-title-bar__helm-count">{helm.questions}</span>
                    ) : null}
                    {helm.binding === undefined ? null : <KbdCmd shortcut={helm.binding} />}
                  </button>
                )}
              </div>
            )}
          </Popover>
        </div>
      </div>
    </div>
  );
}

/** The picker's list, flat: the same entries its dropdown draws, one row each. */
function RepositoryRows({ entries, onSelect, manifest }: NonNullable<TitleBarProps["repositoryMenu"]>) {
  return (
    <div className="armada-title-bar__menu-repositories" role="group" aria-label="Repositories">
      {entries.map((entry) =>
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
