// The Cockpit's stage while Fleet serves no repository: a start menu of three starts (`StartMenu.tsx`). A repository already on this Mac is the folder dialog
// at once; a new project is a name and a place; Claude Code's own sessions are the folders it kept them for, ticked to add. Each ends the same way: the Cockpit
// with a new Job open on the repository, empty, its message box focused, and where the runs are kept as the Job's own card (`RunsPlaceControl.tsx`), which
// asks nothing first. A Job starts on any folder, a Manifest or not. Fleet lists the folder before the Job starts on it, which swaps this stage for the Cockpit,
// so nothing here reads this component's state after the first folder is listed.

import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Folder, FolderOpen, GitBranch } from "lucide-react";
import { Button, Checkbox, Input, KbdChord, Stamp, Switch, Tooltip, useKept } from "@armada/components";
import type { ImportFolder } from "@armada/protocol";

import { refusalWords } from "@armada/screens/src/refusal-words";

import { addRepository, chooseFolder, createProject, installAgentMod, listImportFolders } from "./commands";
import { claudeFolderOf, defaultParent, homeRelative, rootOfImport } from "./run-places";
import { useSessionsDraft } from "./sessions-draft";
import { startOn } from "./start-on";
import { StartField, type Preview } from "./StartField";
import { ORDERS, StartMenu, type Order } from "./StartMenu";
import "./first-run.css";

const NAME = "first-run:name";

/** What a folder name may hold, so the path made of it needs no escaping. */
const NAMED = /^[A-Za-z0-9._-]+$/;

/** Where the person is: the three starts, or one of the two that ask more. Open a project asks nothing but the folder dialog. */
type Stage = "choose" | "new" | "found";

export function FirstRun({ onTell }: { onTell?: ((sentence: string) => void) | undefined }) {
  const draft = useSessionsDraft();
  const [stage, setStage] = useState<Stage>("choose");
  const [last, setLast] = useState(0);
  // The start the focus is on, which the battlefield behind the menu previews.
  const [on, setOn] = useState(0);
  const root = useRef<HTMLElement>(null);

  const [name, setName, clearName] = useKept<string>(NAME, "");
  const [parent, setParent] = useState<string | null>(null);
  const [making, setMaking] = useState(false);

  // The folders Claude Code kept sessions for: read once as the stage shows, since both the third start and the place a new project is made read them.
  const [listed, setListed] = useState<readonly ImportFolder[] | null>(null);
  const [unlisted, setUnlisted] = useState<string | undefined>();
  const [ticked, setTicked] = useState<ReadonlySet<string>>(new Set());
  const [mod, setMod] = useState(false);
  const [adding, setAdding] = useState(false);

  useEffect(() => {
    let live = true;
    void listImportFolders().then((answer) => {
      if (!live) return;
      if (!answer.ok) return setUnlisted(refusalWords(answer.outcome));
      const here = answer.value.folders.filter((one) => one.exists);
      setListed(here);
      setTicked(new Set(here.map((one) => one.path)));
    });
    return () => {
      live = false;
    };
  }, []);

  const found = (listed ?? []).map(claudeFolderOf);
  const where = parent ?? defaultParent((listed ?? []).map((one) => one.path));

  /** The place a new project is made in. */
  async function place(): Promise<void> {
    const chosen = await chooseFolder();
    if (chosen !== null) setParent(chosen);
    root.current?.querySelector<HTMLInputElement>("input")?.focus();
  }

  /**
   * Lists the folders and opens a Job on the first, empty and ready to type in, with the place its runs are kept as the Job's own card.
   * Fleet lists the first folder before the Job starts on it, which swaps this stage for the Cockpit. A folder Fleet already serves, a new project's, is not
   * added again.
   */
  async function open(roots: readonly string[], served = false): Promise<void> {
    const [first, ...rest] = roots;
    if (draft === undefined || first === undefined) return;
    const added = served ? undefined : await addRepository(first);
    if (added !== undefined && added.state !== "located") return onTell?.(added.state === "refused" ? added.saying : "Folder not added");
    await startOn(draft, added === undefined ? first : added.repository.root);
    for (const path of rest) {
      const more = await addRepository(path);
      if (more.state !== "located") onTell?.(more.state === "refused" ? more.saying : "Folder not added");
    }
  }

  async function create(): Promise<void> {
    if (!NAMED.test(name) || making) return;
    setMaking(true);
    const made = await createProject(where, name);
    setMaking(false);
    if (made.state !== "located") return onTell?.(made.state === "refused" ? made.saying : "Project not made");
    clearName();
    await open([made.repository.root], true);
  }

  /** The folders ticked, as Fleet serves them: a folder inside a repository is that repository, once. */
  const chosen = (): string[] => [...new Set((listed ?? []).filter((one) => ticked.has(one.path)).map(rootOfImport))];

  /** The mod's install is the press on the switch and Add; Fleet refuses one that was not asked for. It takes minutes, so the Job does not wait on it. */
  async function install(): Promise<void> {
    const done = await installAgentMod();
    if (!done.ok) return onTell?.(refusalWords(done.outcome));
    // An install that went needs no word: the terminal sessions are Jobs. One that stopped says in the CLI's own words.
    if (done.value.result === "failed") onTell?.(`Mod not installed: ${done.value.said}`);
  }

  /** Adds the ticked folders, and opens a Job on the first. With none ticked, only the mod is installed. */
  async function add(): Promise<void> {
    if (adding) return;
    const roots = chosen();
    if (roots.length === 0 && !mod) return;
    setAdding(true);
    if (roots.length === 0) {
      await install();
      return setAdding(false);
    }
    if (mod) void install();
    await open(roots);
    setAdding(false);
  }

  /** The folder dialog. A folder opens the Job; cancelling leaves the menu as it was. */
  async function choose(): Promise<void> {
    const picked = await chooseFolder();
    if (picked !== null) await open([picked]);
  }

  const pick = (to: Order["to"], from: number) => {
    setLast(from);
    if (to === "folder") void choose();
    else setStage(to);
  };
  // Importing is off where Fleet could not read the folders, or has none.
  const none = unlisted ?? (listed !== null && listed.length === 0 ? "No sessions to import" : undefined);
  const starts: readonly Order[] = ORDERS.map((one) => (one.to !== "found" || none === undefined ? one : { ...one, off: none }));

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
                  <Stamp still first="age" at={one.lastAt} className="armada-first-run__age" />
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
