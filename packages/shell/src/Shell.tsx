// Bridge's frame: the title row with the Cockpit window's tabs, the panel the journeys mount in, and Helm's dock. The drawn
// shell, wired to what Fleet actually serves.
//
// **There is no rail.** The places are the title row's tabs and the System sidebar (`surfaces.ts`), the repository picker is the
// top of the deck's left column, and Fleet's panel is a popover on the title row's dot. #2301.
//
// # The roster is what exists
//
// `surfaces.ts` is where the destinations live. A destination that is not built is not listed: a disabled row would be a
// promise Armada does not keep.
//
// # One region of the drawing has no source, and is left out
//
// `today ~$4.80`: nothing measures spend, and it is not drawn as a labelled blank, because a label with nothing under it
// reads as a value that failed to load.
//
// # The picker and Dispatch live in the title row, not here
//
// #1087 put `Dispatch` in the row `TheShell` draws over the traffic lights. The picker's entries reach the title row's narrow
// menu as `repositoryMenu`, and the deck draws the picker itself.
//
// # No screen draws a page head, #1090
//
// A screen's own controls sit at the top of its content; `App.tsx` is where that moved.

import { FileCog } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import {
  isPressed,
  keyFor,
  TheShell,
  useAccount,
  useKeyBindings,
  type FleetPanelProps,
  type TheShellProps,
} from "@armada/components";

import type { Connection } from "@armada/protocol";
import type { RepositorySummary } from "@armada/protocol";
import { useDockWidth } from "./dock-width";
// The dock folds to a sheet, and job detail's inspector folds to a sheet, at `--layout-breakpoint`. One reader, in `floor.ts`, so the
// two cannot answer a resize a pixel apart.
import { useNarrow } from "./floor";
import { useRecentRepositories } from "./recent-repositories";
import { ADD, ALL, NEW, repositoryEntries, rootOf } from "./repository-entries";
import { SURFACE, SURFACES } from "./surfaces";

export type ShellProps = {
  connection: Connection;
  /** Every repository Fleet serves, set up or not. One entry is why the drawing shows the control. */
  repositories: readonly RepositorySummary[];
  /** Whether Fleet has answered the listing, so an empty one says nothing is set up rather than not read. */
  listed?: boolean;
  /** The picked repository's root, or `null` for All repositories, which is where Bridge opens. */
  scope: string | null;
  /** A root, or `null` for All repositories. */
  onScope: (root: string | null) => void;
  /** Opens Locate. Absent draws no control. */
  onAddRepository?: () => void;
  /** Opens Locate on a new project. Absent draws no row. */
  onNewProject?: () => void;
  /**
   * Opens the Manifest of whatever the picker has picked — the owner, 28 Sep
   * 2026: *"I should be able to select a manifest from the dropdown and get a
   * button or link or icon or something to open that manifest"*. **Absent
   * draws no control.**
   *
   * Drawn on All repositories too, where there is no one Manifest to open: the
   * surface itself asks which repository, `AskRepository`'s own job, rather
   * than this row growing a second answer to the same question.
   */
  onOpenManifest?: () => void;
  /** The left column's Fleet panel — what the status bar used to draw. */
  fleet: Omit<FleetPanelProps, "narrow">;
  /** Opens the composer. The title row's own Dispatch control — #1087 — beside the Board's own. */
  onCompose: () => void;
  /** Opens the command palette from the title row's search field. */
  onSearch: () => void;
  /** Sleep mode's control, in the title row beside Helm's. Absent draws none. */
  sleep?: TheShellProps["sleep"];
  /** The plan's limits, in the title row. Absent draws none. */
  planUsage?: TheShellProps["planUsage"];
  /** Teach on the title row. Absent draws no menu. */
  onTeach?: TheShellProps["onTeach"];
  /** The title row's centre, before the search trigger. See `TheShell`. */
  places?: TheShellProps["places"];
  /** The title row's word in place of the lockup. See `TheShell`. */
  wordmark?: TheShellProps["wordmark"];
  /** The title row's read-only name of the picked repository. See `TheShell`. */
  repository?: TheShellProps["repository"];
  /** The mark on a repository's row in the title row's list, and the act → runs on it. Bridge builds it: the places runs are kept are the app's to read. */
  repositoryTrail?: (root: string, label: string) => { node: ReactNode; open: () => void } | undefined;
  /** Goes to a surface, by its id in `surfaces.ts`: the menu's Settings. */
  onSurface?: (surfaceId: string) => void;
  /**
   * How many Helm asks are waiting inside `helm` — #1519. The folded strip's
   * count is every question waiting on a person, and an ask drawn down in the
   * conversation is still one of them; without this the strip would say nothing
   * was waiting while a reply sat held open.
   */
  asking?: number;
  /** A call that lives in Helm is waiting: the title row's Helm button pulses. See `TheShellDock`. */
  helmCalling?: boolean;
  /** Helm's own conversation, under the questions — #944. Bridge builds it; the dock only mounts it. */
  helm?: ReactNode;
  /**
   * Helm's own act in the dock's head, beside Close — *Start fresh*, moved
   * there by the owner on 18 Sep 2026. Bridge builds it, for the same reason
   * `helm` is a node: what the act costs and when Fleet refuses it are the
   * app's to know, not the shell's.
   */
  helmAction?: ReactNode;
  children: ReactNode;
};

export function Shell({
  connection,
  repositories,
  listed = false,
  scope,
  onScope,
  onAddRepository,
  onNewProject,
  onOpenManifest,
  fleet,
  onCompose,
  onSearch,
  sleep,
  planUsage,
  onTeach,
  places,
  wordmark,
  repository,
  repositoryTrail,
  onSurface,
  asking = 0,
  helmCalling = false,
  helm,
  helmAction,
  children,
}: ShellProps) {
  // The chrome draws keys, Helm's among them, so it redraws when Settings → Keyboard moves one.
  useKeyBindings();
  const narrow = useNarrow();
  const account = useAccount();
  const openAccount = () => onSurface?.(SURFACE.account);
  const dock = useDock(narrow);
  const [dockWidth, resizeDock] = useDockWidth();
  const live = connection.state === "connected";
  // The narrow menu's rows: the picker's list.
  const [recent, remember] = useRecentRepositories();
  const pickerEntries = repositoryEntries(repositories, {
    scope,
    all: repositories.length === 0 ? (listed ? "Nothing set up yet" : "No repository read yet") : "All repositories",
    add: onAddRepository !== undefined,
    create: onNewProject !== undefined,
    recent,
  });
  const pick = (picked: string) => {
    if (picked === ADD) return onAddRepository?.();
    if (picked === NEW) return onNewProject?.();
    const root = rootOf(picked);
    if (root !== ALL) remember(root);
    onScope(root === ALL ? null : root);
  };

  return (
    <TheShell
      {...(places === undefined ? {} : { places })}
      {...(wordmark === undefined ? {} : { wordmark })}
      {...(repository === undefined ? {} : { repository })}
      dock={{
        ...dock,
        // The breakpoint alone: the dock folds to its sheet where the title row has no room for its button.
        folded: narrow,
        binding: keyFor("helm"),
        questions: asking,
        ...(helmCalling ? { calling: true } : {}),
        width: dockWidth,
        onResize: resizeDock,
        action: helmAction,
        children: (
          <>
            {helm}
          </>
        ),
      }}
      repositoryMenu={{
        entries: pickerEntries,
        onSelect: pick,
        ...(repositoryTrail === undefined ? {} : { trailOf: (id: string, label: string) => (rootOf(id) === ALL ? undefined : repositoryTrail(rootOf(id), label)) }),
        ...(onOpenManifest === undefined
          ? {}
          : {
              manifest: {
                label: MANIFEST_SAID,
                icon: ManifestGlyph,
                disabled: repositories.length === 0,
                onOpen: onOpenManifest,
              },
            }),
      }}
      menu={{
        ...(account === undefined ? {} : { account: { onOpen: openAccount } }),
        settings: {
          onOpen: () => onSurface?.(SURFACE.settings),
          ...(SETTINGS_SURFACE?.shortcut === undefined ? {} : { shortcut: SETTINGS_SURFACE.shortcut }),
        },
        doctor: { onOpen: () => onSurface?.(SURFACE.doctor) },
      }}
      onSearch={onSearch}
      {...(sleep === undefined ? {} : { sleep })}
      {...(planUsage === undefined ? {} : { planUsage })}
      {...(onTeach === undefined ? {} : { onTeach })}
      onDispatch={onCompose}
      dispatchDisabled={!live}
      fleet={fleet}
    >
      {children}
    </TheShell>
  );
}

/**
 * Manifest's own glyph and word, read off the surface roster rather than
 * retyped — the control beside the picker and the palette's row for the same
 * destination cannot then disagree about which mark means Manifest.
 */
const MANIFEST_SURFACE = SURFACES.find((one) => one.id === SURFACE.manifest);
const ManifestGlyph = MANIFEST_SURFACE?.icon ?? FileCog;
/** Settings, read off the same roster, so the menu's entry and the palette's row share one key. */
const SETTINGS_SURFACE = SURFACES.find((one) => one.id === SURFACE.settings);
const MANIFEST_SAID = `Open the ${MANIFEST_SURFACE?.label ?? "Manifest"}`;

const OPEN_HELM = "armada:open-helm";

/** Opens Helm's dock from anywhere in the window. A window event, since the dock's open state is the shell's own. */
export function askToOpenHelm(): void {
  window.dispatchEvent(new Event(OPEN_HELM));
}

/**
 * Helm's dock: a panel over the content, or a sheet when folded. `⌘J` toggles
 * whichever the width draws, from every surface and from inside a field.
 *
 * **Shut on open, at every width** — the owner, 22 Sep 2026 (#1583). It stood
 * open because it cost the layout nothing to leave it there once a person had
 * arranged around it; over the content it covers what is being read, so Bridge
 * opens with the screen and Helm is a press away.
 */
function useDock(folded: boolean): { open: boolean; onOpen: (open: boolean) => void } {
  // A Helm Session opened from the Sessions list opens the dock by this event.
  const [beside, setBeside] = useState(false);
  const [sheet, setSheet] = useState(false);

  // A sheet left open would cover the work again the next time the window narrows.
  useEffect(() => {
    if (!folded) setSheet(false);
  }, [folded]);

  const open = folded ? sheet : beside;
  const onOpen = folded ? setSheet : setBeside;
  const toggle = useRef(() => onOpen(!open));
  toggle.current = () => onOpen(!open);
  const opens = useRef(onOpen);
  opens.current = onOpen;
  useEffect(() => {
    const heard = (): void => opens.current(true);
    window.addEventListener(OPEN_HELM, heard);
    return () => window.removeEventListener(OPEN_HELM, heard);
  }, []);

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (event.repeat || !isPressed("helm", event)) return;
      event.preventDefault();
      toggle.current();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, []);

  return { open, onOpen };
}

