// The Cockpit's stage while Fleet serves no repository: a folder and a request, and Start. A Session starts on any folder, a Manifest or not;
// the folder is made a repository by Start. Nothing here explains itself. The Session opens before the folder is listed, so the swap to the
// Cockpit that listing causes finds it open. **Mock only so far**: Fleet makes no repository of a bare folder yet.

import { useRef, useState, type KeyboardEvent } from "react";
import { FolderOpen } from "lucide-react";
import { Button, KbdChord, SendKbd, Textarea, Tooltip, useKept } from "@armada/components";

import { select } from "./cockpit/deck/store";
import { addRepository, chooseFolder, pickRepository } from "./commands";
import { useSessionsDraft } from "./sessions-draft";
import "./first-run.css";

const REQUEST = "first-run:request";
const FOLDER = "first-run:folder";

/** `/Users/name/code/my-app` as the chip writes it. */
const homeRelative = (path: string) => path.replace(/^\/Users\/[^/]+/, "~");

export function FirstRun({ onTell }: { onTell?: ((sentence: string) => void) | undefined }) {
  const draft = useSessionsDraft();
  const [folder, setFolder] = useKept<string | null>(FOLDER, null);
  const [words, setWords] = useState("");
  const [sending, setSending] = useState(false);
  const frame = useRef<HTMLDivElement>(null);

  /** The folder dialog; focus returns to the request, which is where a person was typing. */
  async function choose(): Promise<string | null> {
    const chosen = await chooseFolder();
    if (chosen !== null) setFolder(chosen);
    frame.current?.querySelector<HTMLTextAreaElement>("textarea")?.focus();
    return chosen;
  }

  async function start(at: string): Promise<void> {
    const text = words.trim();
    if (draft === undefined || text === "" || sending) return;
    setSending(true);
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

  /** Start with no folder asks for one, then goes on with the words as they are. */
  async function act(): Promise<void> {
    if (words.trim() === "") return;
    const at = folder ?? (await choose());
    if (at !== null) await start(at);
  }

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "o") {
      event.preventDefault();
      void choose();
    } else if (event.key === "Enter" && !event.shiftKey && !event.altKey) {
      event.preventDefault();
      void act();
    }
  };

  return (
    <section className="armada-first-run" aria-label="First Job" onKeyDown={keys}>
      <div className="armada-first-run__folder">
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
    </section>
  );
}
