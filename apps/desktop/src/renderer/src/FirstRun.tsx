// The Cockpit's stage while Fleet serves no repository: a start menu of three starts (`StartMenu.tsx`). A repository already on this Mac is the folder dialog
// at once; a new project is a name and a place; Claude Code's own sessions are the folders it kept them for, ticked to add. Each ends the same way: the Cockpit
// with a new Job open on the repository, empty, its message box focused, and where the runs are kept as the Job's own line (`RunsPlaceLine.tsx`), which
// asks nothing first. A Job starts on any folder, a Manifest or not; the folder is listed as a repository as the Job opens. The Job opens before the
// folder is listed, so the swap to the Cockpit that listing causes finds it open. **Mock only so far**: Fleet makes no repository of a bare folder,
// no folder of a new project and finds no Claude Code session yet, so the last two starts are offered only where the Jobs draft serves `begin`.

import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Folder, FolderOpen, GitBranch } from "lucide-react";
import { Button, Checkbox, Input, KbdChord, Switch, Tooltip, useKept } from "@armada/components";
import type { ClaudeFolder } from "@armada/screens/src/draft/sessions";

import { select } from "./cockpit/deck/store";
import { addRepository, chooseFolder, pickRepository } from "./commands";
import { askComposerFocus } from "./session-focus";
import { useSessionsDraft } from "./sessions-draft";
import { StartField, type Preview } from "./StartField";
import { ORDERS, StartMenu, type Order } from "./StartMenu";
import "./first-run.css";

const NAME = "first-run:name";

/** `/Users/name/code/my-app` as the chip writes it. */
const homeRelative = (path: string) => path.replace(/^\/Users\/[^/]+/, "~");

/** What a folder name may hold, so the path made of it needs no escaping. */
const NAMED = /^[A-Za-z0-9._-]+$/;

/** Where the person is: the three starts, or one of the two that ask more. Open a project asks nothing but the folder dialog. */
type Stage = "choose" | "new" | "found";

/** The age of a last session as one figure; the tooltip says what it is. */
function age(at: string): string {
  const minutes = Math.max(0, Math.round((Date.now() - Date.parse(at)) / 60_000));
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 60 * 24) return `${Math.floor(minutes / 60)}h`;
  return `${Math.floor(minutes / (60 * 24))}d`;
}

export function FirstRun({ onTell }: { onTell?: ((sentence: string) => void) | undefined }) {
  const draft = useSessionsDraft();
  const begin = draft?.begin;
  const [stage, setStage] = useState<Stage>("choose");
  const [last, setLast] = useState(0);
  // The start the focus is on, which the battlefield behind the menu previews.
  const [on, setOn] = useState(0);
  const root = useRef<HTMLElement>(null);

  const [name, setName, clearName] = useKept<string>(NAME, "");
  const [parent, setParent] = useState<string | null>(null);
  const [making, setMaking] = useState(false);

  const [found, setFound] = useState<readonly ClaudeFolder[] | null>(null);
  const [ticked, setTicked] = useState<ReadonlySet<string>>(new Set());
  const [mod, setMod] = useState(false);
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    if ((stage !== "found" && stage !== "choose") || begin === undefined || found !== null) return;
    let live = true;
    void begin.found().then((all) => {
      if (!live) return;
      setFound(all);
      setTicked(new Set(all.map((one) => one.path)));
    });
    return () => {
      live = false;
    };
  }, [stage, begin, found]);

  const where = parent ?? begin?.home ?? null;

  /** The place a new project is made in. */
  async function place(): Promise<void> {
    const chosen = await chooseFolder();
    if (chosen !== null) setParent(chosen);
    root.current?.querySelector<HTMLInputElement>("input")?.focus();
  }

  /**
   * Lists the folders and opens a Job on the first, empty and ready to type in, with the place its runs are kept as the Job's own line for all of them.
   * The Job is open and selected before the first folder is listed, which swaps this stage for the Cockpit.
   */
  async function open(roots: readonly string[]): Promise<void> {
    const [first, ...rest] = roots;
    if (draft === undefined || first === undefined) return;
    const id = await draft.start(undefined, first);
    if (id === undefined) return;
    draft.runs?.asking?.begin(id, roots);
    askComposerFocus(id);
    // Selected on the deck's own store, which the Cockpit reads when listing the folder swaps it in for this stage.
    select({ kind: "session", key: `session:${id}` });
    const added = await addRepository(first);
    if (added.state !== "located") return onTell?.(added.state === "refused" ? added.saying : "Folder not added");
    pickRepository(added.repository.root);
    for (const path of rest) {
      const more = await addRepository(path);
      if (more.state !== "located") onTell?.(more.state === "refused" ? more.saying : "Folder not added");
    }
  }

  async function create(): Promise<void> {
    if (begin === undefined || where === null || !NAMED.test(name) || making) return;
    setMaking(true);
    const made = await begin.create(where, name);
    setMaking(false);
    clearName();
    await open([made]);
  }

  /** Adds the ticked folders, and opens the Job on the first. With none ticked, only the mod is installed. */
  async function add(): Promise<void> {
    if (begin === undefined || adding) return;
    const paths = (found ?? []).filter((one) => ticked.has(one.path)).map((one) => one.path);
    if (paths.length === 0 && !mod) return;
    setAdding(true);
    await begin.adopt(paths, mod);
    await open(paths);
    setAdding(false);
  }

  /** The folder dialog. A folder opens the Job; cancelling leaves the menu as it was. */
  async function choose(): Promise<void> {
    const chosen = await chooseFolder();
    if (chosen !== null) await open([chosen]);
  }

  const pick = (to: Order["to"], from: number) => {
    setLast(from);
    if (to === "folder") void choose();
    else setStage(to);
  };
  const off = begin === undefined ? "Fleet does not offer this" : undefined;
  const starts: readonly Order[] = ORDERS.map((one) => (one.to === "folder" || off === undefined ? one : { ...one, off }));

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape" && stage !== "choose") {
      event.preventDefault();
      event.stopPropagation();
      setStage("choose");
    } else if (stage === "choose") {
      const at = Number(event.key) - 1;
      const start = starts[at];
      if (start !== undefined && start.off === undefined && !event.metaKey && !event.ctrlKey && !event.altKey) {
        event.preventDefault();
        pick(start.to, at);
      } else if (["j", "k", "ArrowDown", "ArrowUp", "ArrowRight", "ArrowLeft"].includes(event.key)) {
        event.preventDefault();
        const all = [...(root.current?.querySelectorAll<HTMLButtonElement>(".armada-start__order:not(:disabled)") ?? [])];
        const now = all.indexOf(document.activeElement as HTMLButtonElement);
        const forward = event.key === "j" || event.key === "ArrowDown" || event.key === "ArrowRight";
        all[(Math.max(now, 0) + (forward ? 1 : all.length - 1)) % all.length]?.focus();
      }
    } else if (stage === "new") {
      if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
        event.preventDefault();
        void create();
      }
    } else if (event.key === "Enter" && !(event.target instanceof HTMLButtonElement)) {
      event.preventDefault();
      void add();
    } else if (["j", "k", "ArrowDown", "ArrowUp"].includes(event.key)) {
      event.preventDefault();
      const all = [...(root.current?.querySelectorAll<HTMLInputElement>("input") ?? [])];
      const now = all.indexOf(document.activeElement as HTMLInputElement);
      all[Math.min(all.length - 1, Math.max(0, now + (event.key === "j" || event.key === "ArrowDown" ? 1 : -1)))]?.focus();
    }
  };

  const back = (
    <Button variant="ghost" size="sm" onClick={() => setStage("choose")}>
      Back
      <KbdChord keys={["Esc"]} aria-hidden />
    </Button>
  );

  const newStage = (
    <>
      <div className="armada-first-run__folder">{back}</div>
      <div className="armada-first-run__frame">
        <div className="armada-first-run__where">
          <Tooltip label="Change location">
            <button type="button" className="armada-first-run__chip" aria-label={`Location ${where === null ? "" : homeRelative(where)}`.trim()} data-chosen="" onClick={() => void place()}>
              <FolderOpen size={12} strokeWidth={2} aria-hidden />
              <span className="armada-first-run__name">{where === null ? "" : homeRelative(where)}</span>
            </button>
          </Tooltip>
          <span className="armada-first-run__slash" aria-hidden>
            /
          </span>
          <Input aria-label="Name" mono autoFocus value={name} onChange={(event) => setName(event.target.value)} />
        </div>
        <div className="armada-first-run__acts">
          <Button variant="primary" size="sm" pending={making} disabled={!NAMED.test(name)} onClick={() => void create()}>
            Create
          </Button>
        </div>
      </div>
    </>
  );

  const foundStage = (
    <>
      <div className="armada-first-run__folder">{back}</div>
      <div className="armada-first-run__frame">
        <div role="group" aria-label="Claude Code sessions" className="armada-first-run__found">
          {(found ?? []).map((one, at) => (
            <Checkbox
              key={one.path}
              autoFocus={at === 0}
              checked={ticked.has(one.path)}
              onChange={(event) => setTicked((was) => (event.target.checked ? new Set([...was, one.path]) : new Set([...was].filter((it) => it !== one.path))))}
            >
              <span className="armada-first-run__row">
                <span className="armada-first-run__path">{homeRelative(one.path)}</span>
                <Tooltip label="Last session">
                  <span className="armada-first-run__age">{age(one.lastAt)}</span>
                </Tooltip>
                <Tooltip label={one.git ? "Git repository" : "Not a git repository"}>
                  <span className="armada-first-run__git" role="img" aria-label={one.git ? "Git repository" : "Not a git repository"} data-git={one.git ? "" : undefined}>
                    {one.git ? <GitBranch size={12} strokeWidth={2} aria-hidden /> : <Folder size={12} strokeWidth={2} aria-hidden />}
                  </span>
                </Tooltip>
              </span>
            </Checkbox>
          ))}
        </div>
        <Switch checked={mod} onChange={(event) => setMod(event.target.checked)} description="Terminal Claude Code sessions appear as Jobs">
          Install the Armada mod
        </Switch>
        <div className="armada-first-run__acts">
          <Button variant="primary" size="sm" pending={adding} disabled={ticked.size === 0 && !mod} onClick={() => void add()}>
            Add
          </Button>
        </div>
      </div>
    </>
  );

  const preview: Preview = stage === "choose" ? (starts[on]?.to ?? "none") : stage;

  return (
    <>
      <StartField preview={preview} folders={(found ?? []).map((one) => one.path)} />
      <section ref={root} className="armada-first-run" data-stage={stage} aria-label="First Job" onKeyDown={keys}>
        {stage === "choose" ? (
          <StartMenu at={on} last={last} orders={starts} onPick={pick} onFocus={setOn} />
        ) : stage === "new" ? (
          newStage
        ) : (
          foundStage
        )}
      </section>
    </>
  );
}
