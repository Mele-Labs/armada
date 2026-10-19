// The Cockpit's stage while Fleet serves no repository: a start menu of three starts (`StartMenu.tsx`). A repository already on this Mac is a folder and a request,
// and Start; a new project is a name and a place, then the same request; Claude Code's own sessions are the folders it kept them for, ticked to add.
// A Job starts on any folder, a Manifest or not; the folder is made a repository by Start. Each start asks once where the repository's runs are kept,
// one step after the folder, the name or the folders are chosen and before the request. Nothing here explains itself. The Job opens before the
// folder is listed, so the swap to the Cockpit that listing causes finds it open. **Mock only so far**: Fleet makes no repository of a bare folder,
// no folder of a new project and finds no Claude Code session yet, so the last two starts are offered only where the Jobs draft serves `begin`.

import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { Folder, FolderOpen, GitBranch } from "lucide-react";
import { Button, Checkbox, Input, KbdChord, RunsPlaces, SendKbd, Switch, Textarea, Tooltip, useKept, type RunsPlace } from "@armada/components";
import type { ClaudeFolder } from "@armada/screens/src/draft/sessions";

import { select } from "./cockpit/deck/store";
import { addRepository, chooseFolder, pickRepository } from "./commands";
import { useSessionsDraft } from "./sessions-draft";
import { StartField, type Preview } from "./StartField";
import { ORDERS, StartMenu, type Order } from "./StartMenu";
import "./first-run.css";

const REQUEST = "first-run:request";
const FOLDER = "first-run:folder";
const NAME = "first-run:name";

/** `/Users/name/code/my-app` as the chip writes it. */
const homeRelative = (path: string) => path.replace(/^\/Users\/[^/]+/, "~");

/** What a folder name may hold, so the path made of it needs no escaping. */
const NAMED = /^[A-Za-z0-9._-]+$/;

/** Where the person is: the three starts, or one of them, or the place for the runs. `folder` is the request, whichever way the folder came. */
type Stage = "choose" | "folder" | "new" | "found" | "place";

/** Where the runs are kept unless another place is picked. */
const MAC: RunsPlace = { id: "mac" };

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
  const [folder, setFolder] = useKept<string | null>(FOLDER, null);
  const [words, setWords] = useState("");
  const [sending, setSending] = useState(false);
  const frame = useRef<HTMLDivElement>(null);
  const root = useRef<HTMLElement>(null);

  const [name, setName, clearName] = useKept<string>(NAME, "");
  const [parent, setParent] = useState<string | null>(null);
  const [making, setMaking] = useState(false);

  const [found, setFound] = useState<readonly ClaudeFolder[] | null>(null);
  const [ticked, setTicked] = useState<ReadonlySet<string>>(new Set());
  const [mod, setMod] = useState(false);
  const [adding, setAdding] = useState(false);

  // Where each folder's runs are kept, and what the place step goes on with once one is picked.
  const runs = draft?.runs;
  const kept = useRef(new Map<string, RunsPlace>());
  const then = useRef<(place: RunsPlace) => void>(() => undefined);

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

  /** Asks where the runs are kept, then goes on with the pick. Where Fleet keeps none, there is nothing to ask. */
  function ask(go: (place: RunsPlace) => void): void {
    if (runs === undefined) return go(MAC);
    then.current = go;
    setStage("place");
  }

  /** A folder is asked for once, and the request follows. */
  function asked(folderAt: string): void {
    ask((place) => {
      kept.current.set(folderAt, place);
      setStage("folder");
    });
  }

  async function create(): Promise<void> {
    if (begin === undefined || where === null || !NAMED.test(name) || making) return;
    setMaking(true);
    const made = await begin.create(where, name);
    setMaking(false);
    clearName();
    setFolder(made);
    asked(made);
  }

  async function adopt(paths: readonly string[], place: RunsPlace): Promise<void> {
    if (begin === undefined) return;
    setStage("found");
    setAdding(true);
    await begin.adopt(paths, mod);
    for (const path of paths) {
      runs?.place(path, place);
      const added = await addRepository(path);
      if (added.state !== "located") onTell?.(added.state === "refused" ? added.saying : "Folder not added");
    }
    setAdding(false);
  }

  /** The place is asked once for all the folders being added. */
  function add(): void {
    if (begin === undefined || adding) return;
    const paths = (found ?? []).filter((one) => ticked.has(one.path)).map((one) => one.path);
    if (paths.length === 0 && !mod) return;
    if (paths.length === 0) return void adopt(paths, MAC);
    ask((place) => void adopt(paths, place));
  }

  /** The folder dialog; the place is asked next, and focus returns to the request, which is where a person was typing. */
  async function choose(): Promise<string | null> {
    const chosen = await chooseFolder();
    if (chosen !== null) {
      setFolder(chosen);
      asked(chosen);
    }
    frame.current?.querySelector<HTMLTextAreaElement>("textarea")?.focus();
    return chosen;
  }

  async function start(at: string): Promise<void> {
    const text = words.trim();
    if (draft === undefined || text === "" || sending) return;
    setSending(true);
    runs?.place(at, kept.current.get(at) ?? MAC);
    const id = await draft.start(undefined, at, text, {});
    if (id === undefined) return setSending(false);
    void draft.send(id, { text, files: [], sketches: [], tags: [] });
    setWords("");
    // Selected on the deck's own store, which the Cockpit reads when listing the folder swaps it in for this prompt.
    select({ kind: "session", key: `session:${id}` });
    const added = await addRepository(at);
    if (added.state !== "located") return onTell?.(added.state === "refused" ? added.saying : "Folder not added");
    pickRepository(added.repository.root);
  }

  /** Start with no folder asks for one and then where its runs are kept; the words stay as they are for Start to go on with. */
  async function act(): Promise<void> {
    if (words.trim() === "") return;
    if (folder === null) await choose();
    else await start(folder);
  }

  const pick = (to: Stage, from: number) => {
    setLast(from);
    setStage(to);
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
    } else if (stage === "folder") {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "o") {
        event.preventDefault();
        void choose();
      } else if (event.key === "Enter" && !event.shiftKey && !event.altKey && event.target instanceof HTMLTextAreaElement) {
        event.preventDefault();
        void act();
      }
    } else if (stage === "new") {
      if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
        event.preventDefault();
        void create();
      }
    } else if (stage === "place") {
      // The places take their own keys.
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

  const folderStage = (
    <>
      <div className="armada-first-run__folder">
        {back}
        <Tooltip label={folder ?? "Choose a folder"}>
          <button type="button" className="armada-first-run__chip" aria-label={folder === null ? "Folder" : `Folder ${homeRelative(folder)}`} data-chosen={folder === null ? undefined : ""} onClick={() => void choose()}>
            <FolderOpen size={12} strokeWidth={2} aria-hidden />
            {folder === null ? null : <span className="armada-first-run__name">{homeRelative(folder)}</span>}
            <KbdChord keys={["⌘", "O"]} aria-hidden />
          </button>
        </Tooltip>
      </div>
      <div ref={frame} className="armada-first-run__frame">
        <Textarea
          aria-label="Request"
          keep={REQUEST}
          rows={4}
          autoFocus
          value={words}
          onChange={(event) => setWords(event.target.value)}
        />
        <div className="armada-first-run__acts">
          <SendKbd available={words.trim() !== ""} />
          <Button variant="primary" size="sm" pending={sending} disabled={words.trim() === ""} onClick={() => void act()}>
            Start
          </Button>
        </div>
      </div>
    </>
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

  const placeStage = (
    <>
      <div className="armada-first-run__folder">{back}</div>
      <div className="armada-first-run__frame">
        <span className="armada-first-run__title">Where runs are kept</span>
        <RunsPlaces synced={runs?.synced ?? []} profiles={runs?.profiles ?? []} onPick={(place) => then.current(place)} onClose={() => setStage("choose")} />
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

  const preview: Preview = stage === "choose" ? (starts[on]?.to ?? "none") : stage === "place" ? (starts[last]?.to ?? "none") : stage;

  return (
    <>
      <StartField preview={preview} folders={(found ?? []).map((one) => one.path)} />
      <section ref={root} className="armada-first-run" data-stage={stage} aria-label="First Job" onKeyDown={keys}>
        {stage === "choose" ? (
          <StartMenu at={on} last={last} orders={starts} onPick={pick} onFocus={setOn} />
        ) : stage === "folder" ? (
          folderStage
        ) : stage === "new" ? (
          newStage
        ) : stage === "place" ? (
          placeStage
        ) : (
          foundStage
        )}
      </section>
    </>
  );
}
