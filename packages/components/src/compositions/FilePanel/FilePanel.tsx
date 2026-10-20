import { FolderOpen, SquareArrowOutUpRight } from "lucide-react";
import { useEffect } from "react";

import { Alert } from "../../primitives/Alert/Alert";
import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { CodeEditor } from "./CodeEditor";

/**
 * Any file, open in the side panel: text in a code editor where it is edited and saved, a picture just shown.
 *
 * **Review and edit are one place.** The head names the file and carries the two acts that leave Bridge for it, and the
 * body is the file. A save that finds the file changed on disk does not write: it draws both and asks which stands, as
 * the Manifest's file view does. The unsaved mark is a dot with a tooltip, and nothing says it in words.
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
};

export function FilePanel({
  name,
  path,
  type,
  text,
  imageSrc,
  dirty,
  saving = false,
  onChange,
  onSave,
  onOpenInEditor,
  onShowInFinder,
  onClose,
  moved = null,
  onKeepMine,
  onTakeDisk,
  refused,
  floor = false,
  back,
}: FilePanelProps) {
  // ⌘S on the window while the panel is up, so it saves with focus on the editor or on the head.
  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey || event.key.toLowerCase() !== "s") return;
      event.preventDefault();
      if (dirty && !saving && moved === null) onSave?.();
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  }, [dirty, saving, moved, onSave]);

  const picture = type.startsWith("image/");
  return (
    <Sheet
      kind="side-panel"
      open
      floating
      floor={floor}
      title={name}
      subtitle={<span className="armada-file-panel__path">{path}</span>}
      size="panel"
      {...(back === undefined ? {} : { back: { label: back.label, tooltip: back.label, onBack: back.onBack } })}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
      controls={
        <span className="armada-file-panel__acts" role="group" aria-label="Acts">
          {dirty ? (
            <Tooltip label="Unsaved">
              <span className="armada-file-panel__unsaved" role="img" aria-label="Unsaved" tabIndex={0} />
            </Tooltip>
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
      }
    >
      <div className="armada-file-panel">
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
              <img src={imageSrc} alt={name} />
            </div>
          )
        ) : text === undefined ? null : (
          <div className="armada-file-panel__well" data-whole>
            <CodeEditor name={name} value={text} {...(onChange === undefined ? {} : { onChange })} />
          </div>
        )}
      </div>
    </Sheet>
  );
}
