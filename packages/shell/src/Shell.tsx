// Bridge's frame: the title row with the Cockpit window's tabs, the panel the journeys mount in, and the Operator's dock. The drawn
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
// The dock folds to a sheet, and job detail's inspector folds to a sheet, at `--layout-breakpoint`. One reader, in `floor.ts`, so the
// two cannot answer a resize a pixel apart.
import { useNarrow } from "./floor";
import { useRecentRepositories } from "./recent-repositories";
import {
  ADD,
  ALL,
  NEW,
  NEW_WORKSPACE,
  WORKSPACE,
  repositoryEntries,
  rootOf,
} from "./repository-entries";
import type { PickerWorkspace } from "./repository-entries";
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
  /** The Workspaces, each listed as a heading over its repositories. */
  workspaces?: readonly PickerWorkspace[];
  /** Opens a Workspace's page, or a new one's where `null`. Absent lists no New Workspace and draws no gear. */
  onOpenWorkspace?: (id: string | null) => void;
  /** The Workspace the window is scoped to, if any. Its heading shows selected. */
  workspaceScope?: string | null;
  /** A Workspace heading was picked: the window shows its repositories. */
  onScopeWorkspace?: (id: string) => void;
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
  repositoryTrail?: (
    root: string,
    label: string,
  ) =>
    | {
        node: ReactNode;
        open: () => void;
      }
    | undefined;
  /** Goes to a surface, by its id in `surfaces.ts`: the menu's Settings. */
  onSurface?: (surfaceId: string) => void;
  /** The Operator's panel, mounted in the dock. Bridge builds it. **Absent draws no dock and no title row button, and `⌘J` does nothing.** */
  operator?: ReactNode;
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
  workspaces,
  onOpenWorkspace,
  workspaceScope = null,
  onScopeWorkspace,
  onOpenManifest,
  fleet,
  onCompose,
  onSearch,
  planUsage,
  onTeach,
  places,
  wordmark,
  repository,
  repositoryTrail,
  onSurface,
  operator,
  children,
}: ShellProps) {
  // The chrome draws keys, the Operator's among them, so it redraws when Settings → Keyboard moves one.
  useKeyBindings();
  const narrow = useNarrow();
  const account = useAccount();
  const openAccount = () => onSurface?.(SURFACE.account);
  const dock = useDock(narrow, operator !== undefined);
  const live = connection.state === "connected";
  // The narrow menu's rows: the picker's list.
  const [recent, remember] = useRecentRepositories();
  const pickerEntries = repositoryEntries(repositories, {
    scope,
    all:
      repositories.length === 0
        ? listed
          ? "Nothing set up yet"
          : "No repository read yet"
        : "All repositories",
    add: onAddRepository !== undefined,
    create: onNewProject !== undefined,
    recent,
    ...(workspaces === undefined
      ? {}
      : {
          workspaces,
        }),
    newWorkspace: onOpenWorkspace !== undefined,
    workspaceScope,
  });
  const pick = (picked: string) => {
    if (picked === ADD) return onAddRepository?.();
    if (picked === NEW) return onNewProject?.();
    if (picked === NEW_WORKSPACE) return onOpenWorkspace?.(null);
    if (picked.startsWith(WORKSPACE)) return onScopeWorkspace?.(picked.slice(WORKSPACE.length));
    const root = rootOf(picked);
    if (root !== ALL) remember(root);
    onScope(root === ALL ? null : root);
  };

  return (
    <TheShell
      {...(places === undefined
        ? {}
        : {
            places,
          })}
      {...(wordmark === undefined
        ? {}
        : {
            wordmark,
          })}
      {...(repository === undefined
        ? {}
        : {
            repository,
          })}
      {...(operator === undefined
        ? {}
        : {
            dock: {
              ...dock,
              // The breakpoint alone: the dock folds to its sheet where the title row has no room for its button.
              folded: narrow,
              binding: keyFor("operator"),
              span: "full" as const,
              children: <>{operator}</>,
            },
          })}
      repositoryMenu={{
        entries: pickerEntries,
        onSelect: pick,
        ...(onOpenWorkspace === undefined
          ? {}
          : {
              onOpenWorkspace,
            }),
        ...(repositoryTrail === undefined
          ? {}
          : {
              trailOf: (id: string, label: string) =>
                rootOf(id) === ALL ? undefined : repositoryTrail(rootOf(id), label),
            }),
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
        ...(account === undefined
          ? {}
          : {
              account: {
                onOpen: openAccount,
              },
            }),
        settings: {
          onOpen: () => onSurface?.(SURFACE.settings),
          ...(SETTINGS_SURFACE?.shortcut === undefined
            ? {}
            : {
                shortcut: SETTINGS_SURFACE.shortcut,
              }),
        },
        doctor: {
          onOpen: () => onSurface?.(SURFACE.doctor),
        },
      }}
      onSearch={onSearch}
      {...(planUsage === undefined
        ? {}
        : {
            planUsage,
          })}
      {...(onTeach === undefined
        ? {}
        : {
            onTeach,
          })}
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

const OPEN_OPERATOR = "armada:open-operator";
const CLOSE_OPERATOR = "armada:close-operator";

/** What asks for the Operator, heard by the app, which draws it as a page: `open` to go to it, `close` to leave it for a Job. */
export function useOperatorAsks(open: () => void, close: () => void): void {
  const latest = useRef({
    open,
    close,
  });
  latest.current = {
    open,
    close,
  };
  useEffect(() => {
    const opened = (): void => latest.current.open();
    const closed = (): void => latest.current.close();
    window.addEventListener(OPEN_OPERATOR, opened);
    window.addEventListener(CLOSE_OPERATOR, closed);
    return () => {
      window.removeEventListener(OPEN_OPERATOR, opened);
      window.removeEventListener(CLOSE_OPERATOR, closed);
    };
  }, []);
}

/** Opens the Operator from anywhere in the window, as the Cockpit's Operator row does. */
export function askToOpenOperator(): void {
  window.dispatchEvent(new Event(OPEN_OPERATOR));
}

/** Shuts the dock from inside it, as a Job opened from the Operator's panel does so the Job is not under it. */
export function askToCloseOperator(): void {
  window.dispatchEvent(new Event(CLOSE_OPERATOR));
}

/**
 * The Operator's dock: a panel over the content, or a sheet when folded. `⌘J` toggles
 * whichever the width draws, from every surface and from inside a field. **With no Operator
 * there is no dock, and `⌘J` is heard by nothing.**
 *
 * **Shut on open, at every width** — the owner, 22 Sep 2026 (#1583). It stood
 * open because it cost the layout nothing to leave it there once a person had
 * arranged around it; over the content it covers what is being read, so Bridge
 * opens with the screen and the Operator is a press away.
 */
function useDock(
  folded: boolean,
  present: boolean,
): {
  open: boolean;
  onOpen: (open: boolean) => void;
} {
  const [beside, setBeside] = useState(false);
  const [sheet, setSheet] = useState(false);

  // A sheet left open would cover the work again the next time the window narrows.
  useEffect(() => {
    if (!folded) setSheet(false);
  }, [
    folded,
  ]);

  const open = folded ? sheet : beside;
  const onOpen = folded ? setSheet : setBeside;
  const toggle = useRef(() => onOpen(!open));
  toggle.current = () => onOpen(!open);
  const opens = useRef(onOpen);
  opens.current = onOpen;
  useEffect(() => {
    const shut = (): void => opens.current(false);
    const show = (): void => opens.current(true);
    window.addEventListener(CLOSE_OPERATOR, shut);
    window.addEventListener(OPEN_OPERATOR, show);
    return () => {
      window.removeEventListener(CLOSE_OPERATOR, shut);
      window.removeEventListener(OPEN_OPERATOR, show);
    };
  }, []);

  useEffect(() => {
    if (!present) return;
    function pressed(event: KeyboardEvent): void {
      if (event.repeat || !isPressed("operator", event)) return;
      event.preventDefault();
      toggle.current();
    }
    window.addEventListener("keydown", pressed);
    return () => window.removeEventListener("keydown", pressed);
  }, [
    present,
  ]);

  return {
    open,
    onOpen,
  };
}
