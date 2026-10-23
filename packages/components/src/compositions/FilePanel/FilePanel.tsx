import { Eye, FolderOpen, SquareArrowOutUpRight } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { KbdBinding } from "../../primitives/Kbd/Kbd";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Switch } from "../../primitives/Switch/Switch";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { readKept, useKept } from "../../keep";
import type { NoteRegion } from "../ViewerNotes/compose";
import { NotableImage } from "../ViewerNotes/NotableImage";
import { useViewerNotes, type NoteAnchor, type ViewerNotes, type ViewerNotesScope } from "../ViewerNotes/ViewerNotes";
import { CodeEditor, type CodeLines, type CodeNoteMark } from "./CodeEditor";
import { MarkdownPreview } from "./MarkdownPreview";

/**
 * Any file, open in the side panel: text in a code editor where it is edited and saved, a picture just shown.
 *
 * **Review and edit are one place.** The head names the file and carries the two acts that leave Bridge for it, and the
 * body is the file. A save that finds the file changed on disk does not write: it draws both and asks which stands, as
 * the Manifest's file view does. Save, with its key, stands in the head only while there is an edit.
 */
export type FilePanelProps = {
  /** The file's name, which is the panel's title and picks the editor's language. */
  name: string;
  /** Where it is. Mono: a path is a fact the system reported. */
  path: string;
  /** `text/…` is edited; `image/…` is shown from `imageSrc`. */
  type: string;
  /** The text, as edited. */
  text?: string;
  /** A picture's address, a blob the renderer made from the bytes. */
  imageSrc?: string;
  /** The text differs from what was read. */
  dirty: boolean;
  saving?: boolean;
  onChange?: (text: string) => void;
  onSave?: () => void;
  onOpenInEditor?: () => void;
  onShowInFinder?: () => void;
  onClose: () => void;
  /** A save met a different file: what is on disk now. */
  moved?: { onDisk: string } | null;
  /** Keep the edit, and write it over what is on disk. */
  onKeepMine?: () => void;
  /** Drop the edit and carry on from what is on disk. */
  onTakeDisk?: () => void;
  /** Why a read or a save failed, facts only. */
  refused?: string;
  /** The window is at its floor. */
  floor?: boolean;
  /** The way back to the item this one was opened from. */
  back?: { label: string; onBack: () => void };
  /** The `kind` of the floating sheet it opens to the left of, which stays open: the Loadout, for a skill's SKILL.md. */
  beside?: string;
  /** The Preview opened, `true`, or closed with it or with the file, `false`: the sheet it sat beside steps aside meanwhile. */
  onPreviewing?: (on: boolean) => void;
  /** Where notes on the file are held, and the Job they go to where it was opened in one. Absent takes none. */
  notes?: ViewerNotesScope;
  /** What a sent note on the file named, shown again as it opens: its lines selected, or its region drawn on the picture. */
  marked?: { lines?: { from: number; to: number }; region?: NoteRegion };
};

/** What the file shows and does, with or without a sheet around it. */
type FileBodyProps = Omit<FilePanelProps, "onClose" | "floor" | "back" | "onOpenInEditor" | "onShowInFinder"> & {
  /** The editor scrolls with a preview beside it: where that one put it, and where a person did. */
  follow?: { to?: number; onScroll: (line: number) => void };
  /** The notes on the text, which the preview beside it shares. */
  noted?: FileNotes;
};

/** The notes on a file's text: the editor and its preview mark the same ones and open the same box. */
type FileNotes = { viewer: ViewerNotes; marks: readonly CodeNoteMark[]; onNote: (at: { x: number; y: number }, lines: CodeLines, drawn: NoteAnchor | undefined, from: HTMLElement | null) => void };

function useFileNotes(path: string, scope: ViewerNotesScope | undefined): FileNotes {
  const from = useRef<HTMLElement | null>(null);
  // The editor's text is in its shadow root, and that is where focus goes back to.
  const viewer = useViewerNotes(scope, from, () => (from.current?.shadowRoot?.querySelector<HTMLElement>("[contenteditable]") ?? from.current)?.focus());
  const marks = viewer.marks.flatMap((one) => (one.about.kind === "file" ? [{ n: one.n, from: one.about.from, to: one.about.to, writing: one.writing }] : []));
  return {
    viewer,
    marks,
    onNote: (at, lines, drawn, element) => {
      // The box opens over whichever of the two was pressed, and focus goes back to it.
      from.current = element;
      viewer.menu(at, { kind: "file", path, ...lines }, drawn);
    },
  };
}

/** ⌘S on the window while a file is up, so it saves with focus on the editor or on the head. */
function useSaveKey({ dirty, saving, moved, onSave }: Pick<FileBodyProps, "dirty" | "saving" | "moved" | "onSave">): void {
  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey || event.key.toLowerCase() !== "s") return;
      event.preventDefault();
      if (dirty && !saving && moved == null) onSave?.();
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  }, [dirty, saving, moved, onSave]);
}

/**
 * Save, only while there is an edit to save, and the two acts that leave Bridge for the file. Nothing stands where
 * Save was once the file is saved. A file met changed on disk draws its own acts, so Save leaves with it.
 */
function FileActs({
  dirty,
  saving = false,
  moved = null,
  onSave,
  onOpenInEditor,
  onShowInFinder,
  preview,
}: Pick<FilePanelProps, "dirty" | "saving" | "moved" | "onSave" | "onOpenInEditor" | "onShowInFinder"> & { preview?: { on: boolean; toggle: () => void } }) {
  return (
    <span className="armada-file-panel__acts" role="group" aria-label="Acts">
      {preview === undefined ? null : (
        <Tooltip label="Preview">
          <Button variant="ghost" size="sm" iconOnly aria-label="Preview" aria-pressed={preview.on} onClick={preview.toggle}>
            <Eye size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      {dirty && moved === null ? (
        <Button variant="primary" size="sm" disabled={saving} onClick={onSave}>
          Save
          <KbdBinding binding="⌘S" />
        </Button>
      ) : null}
      {onOpenInEditor === undefined ? null : (
        <Tooltip label="Open in your editor">
          <Button variant="ghost" size="sm" iconOnly aria-label="Open in your editor" onClick={onOpenInEditor}>
            <SquareArrowOutUpRight size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
      {onShowInFinder === undefined ? null : (
        <Tooltip label="Show in Finder">
          <Button variant="ghost" size="sm" iconOnly aria-label="Show in Finder" onClick={onShowInFinder}>
            <FolderOpen size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
      )}
    </span>
  );
}

function FileBody({ name, path, type, text, imageSrc, saving = false, onChange, moved = null, onKeepMine, onTakeDisk, refused, follow, notes, noted, marked }: FileBodyProps) {
  const picture = type.startsWith("image/");
  const well = useRef<HTMLDivElement>(null);
  return (
    <div className="armada-file-panel" onKeyDown={noted?.viewer.onKeyDown}>
      {refused === undefined ? null : (
        <Alert tone="escalated" title={text === undefined && imageSrc === undefined ? "Not opened" : "Not saved"}>
          {refused}
        </Alert>
      )}
      {moved !== null ? (
        <>
          <Alert tone="caution" title="Changed on disk">
            Your edit is not saved.
          </Alert>
          <div className="armada-file-panel__pair">
            <section className="armada-file-panel__side" aria-label="On disk">
              <span className="armada-file-panel__side-label">On disk</span>
              <div className="armada-file-panel__well">
                <CodeEditor name={`${name} on disk`} value={moved.onDisk} />
              </div>
            </section>
            <section className="armada-file-panel__side" aria-label="Your edit">
              <span className="armada-file-panel__side-label">Your edit</span>
              <div className="armada-file-panel__well">
                <CodeEditor name={`${name} your edit`} value={text ?? ""} {...(onChange === undefined ? {} : { onChange })} />
              </div>
            </section>
          </div>
          <div className="armada-file-panel__moved-acts">
            <Button variant="secondary" disabled={saving} onClick={onTakeDisk}>
              Take disk
            </Button>
            <Button variant="primary" disabled={saving} onClick={onKeepMine}>
              Keep mine
            </Button>
          </div>
        </>
      ) : picture ? (
        imageSrc === undefined ? null : (
          <div className="armada-file-panel__picture">
            <NotableImage src={imageSrc} name={name} path={path} {...(notes === undefined ? {} : { notes })} {...(marked?.region === undefined ? {} : { marked: marked.region })} />
          </div>
        )
      ) : text === undefined ? null : (
        <div ref={well} className="armada-file-panel__well" data-whole>
          <CodeEditor
            name={name}
            value={text}
            {...(onChange === undefined ? {} : { onChange })}
            {...(marked?.lines === undefined ? {} : { reveal: marked.lines })}
            {...(follow === undefined ? {} : { onScroll: follow.onScroll, ...(follow.to === undefined ? {} : { scrollTo: follow.to }) })}
            {...(noted === undefined || !noted.viewer.on ? {} : { notes: noted.marks, onNote: (at: { x: number; y: number }, lines: CodeLines, drawn: NoteAnchor | undefined) => noted.onNote(at, lines, drawn, well.current?.querySelector<HTMLElement>(".armada-code-editor") ?? null) })}
          />
        </div>
      )}
      {noted?.viewer.layer}
    </div>
  );
}

/** Markdown is the one text a rendered view has something to say of. */
const isMarkdown = (name: string, type: string): boolean => type.startsWith("text/") && /\.(md|markdown)$/i.test(name);

/** Where a person's choice to open a markdown file with its preview is kept, for every file and every window. */
const PREVIEW_BY_DEFAULT = "file-panel.preview-by-default";

export function FilePanel({ onClose, floor = false, back, beside, onPreviewing, onOpenInEditor, onShowInFinder, ...file }: FilePanelProps) {
  useSaveKey(file);
  const { name, path, dirty } = file;
  const [previewByDefault, setPreviewByDefault] = useKept<boolean>(PREVIEW_BY_DEFAULT, false);
  const [previewing, setPreviewing] = useState(() => isMarkdown(name, file.type) && readKept<boolean>(PREVIEW_BY_DEFAULT) === true);
  // The line either side was last scrolled to, and which side it was: the other follows it.
  const [lock, setLock] = useState<{ line: number; by: "file" | "preview" } | undefined>(undefined);
  const preview = isMarkdown(name, file.type) ? { on: previewing, toggle: () => setPreviewing((was) => !was) } : undefined;
  const shown = preview?.on === true;
  const noted = useFileNotes(path, file.notes);
  const read = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!shown) return;
    onPreviewing?.(true);
    return () => onPreviewing?.(false);
  }, [shown, onPreviewing]);
  return (
    <>
    <Sheet
      kind="side-panel"
      open
      floating
      {...(beside === undefined ? {} : { beside })}
      bleed
      floor={floor}
      title={name}
      subtitle={
        <Tooltip label={path} asChild>
          <span className="armada-file-panel__path">
            <bdi>{path.replace(/^\/Users\/[^/]+/, "~")}</bdi>
          </span>
        </Tooltip>
      }
      size="panel"
      {...(back === undefined ? {} : { back: { label: back.label, tooltip: back.label, onBack: back.onBack } })}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
      controls={<FileActs dirty={dirty} saving={file.saving} moved={file.moved} onSave={file.onSave} {...(preview === undefined ? {} : { preview })} {...(onOpenInEditor === undefined ? {} : { onOpenInEditor })} {...(onShowInFinder === undefined ? {} : { onShowInFinder })} />}
    >
      <FileBody
        {...file}
        noted={noted}
        {...(previewing && preview !== undefined ? { follow: { onScroll: (line: number) => setLock({ line, by: "file" }), ...(lock?.by === "preview" ? { to: lock.line } : {}) } } : {})}
      />
    </Sheet>
    {preview?.on !== true ? null : (
      <Sheet kind="file-preview" beside="side-panel" open floating floor={floor} title="Preview" size="panel" closeLabel="Close" closeBinding="Esc" controls={<Switch checked={previewByDefault} onChange={(event) => setPreviewByDefault(event.target.checked)}>Open previews by default</Switch>} onClose={previewByDefault ? onClose : preview.toggle}>
        <div ref={read} className="armada-file-panel__read" onKeyDown={noted.viewer.onKeyDown}>
          <MarkdownPreview
            text={file.text ?? ""}
            onScroll={(line) => setLock({ line, by: "preview" })}
            {...(lock?.by === "file" ? { scrollTo: lock.line } : {})}
            {...(!noted.viewer.on ? {} : { notes: noted.marks, onNote: (at: { x: number; y: number }, lines: CodeLines, drawn: NoteAnchor | undefined) => noted.onNote(at, lines, drawn, read.current?.querySelector<HTMLElement>(".armada-markdown-preview") ?? null) })}
          />
        </div>
      </Sheet>
    )}
    </>
  );
}

/**
 * The same file, in a column of the page instead of a sheet: `head` on the left and Save, while there is an edit, and the two
 * acts on the right, the file under it filling what is left. Nothing closes, so Esc is left to the editor.
 */
export function FileView({ head, onOpenInEditor, onShowInFinder, ...file }: Omit<FilePanelProps, "onClose" | "floor" | "back"> & { head: ReactNode }) {
  useSaveKey(file);
  const noted = useFileNotes(file.path, file.notes);
  return (
    <div
      className="armada-file-view"
      onKeyDown={(event) => {
        if (event.key === "Escape") event.stopPropagation();
      }}
    >
      <header className="armada-file-view__head">
        <div className="armada-file-view__who">{head}</div>
        <FileActs dirty={file.dirty} saving={file.saving} moved={file.moved} onSave={file.onSave} {...(onOpenInEditor === undefined ? {} : { onOpenInEditor })} {...(onShowInFinder === undefined ? {} : { onShowInFinder })} />
      </header>
      <FileBody {...file} noted={noted} />
    </div>
  );
}
