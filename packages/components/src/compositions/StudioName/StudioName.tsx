import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";

/**
 * Studio name — a Studio's name where it is drawn, and the way a person gives
 * it one. `docs/concepts/studio.md`, Helm on a Studio; `#1364`.
 *
 * **Editable where it is drawn**, on the list row and on the open Studio. The
 * owner clicked an untitled Studio's name to name it and nothing happened; a
 * rename behind a menu would answer the report and not the gesture.
 *
 * **A heading carries its own Rename; a row's is in the row's split button.**
 * The owner asked for that on 28 Sep 2026, so the whole row could open the
 * Studio — which means the field is opened from outside on a row, and `naming`
 * is that door. `onOpen` is gone with it: nothing inside the name is a press.
 */

export type StudioNameProps = {
  /** The name Fleet holds, or `null` on a Studio nobody has named. */
  name: string | null;
  /** What an untitled Studio is called. The screen's word, not this one's. */
  untitled: string;
  /**
   * Nothing is renamed without this: a Studio reopened read-only, and a window
   * with no connection, both draw the name and no way to change it.
   */
  editable?: boolean;
  /** Drawn as the page's own heading rather than as a row's text. */
  heading?: boolean;
  /**
   * The field is open, asked for by a control outside this one — the list row's
   * split button. **Absent leaves the naming to the Rename beside the name**,
   * which is the heading's own way in.
   */
  naming?: boolean;
  /** Told whenever the field opens or closes, so the outside control keeps up. */
  onNaming?: (naming: boolean) => void;
  /** The name a person settled on. Never called with blank. */
  onRename: (name: string) => void;
  /** Out to Fleet: the field waits rather than taking a second press. */
  saving?: boolean;
  /** Why the last rename did not land, or absent. */
  refused?: string;
};

/** A name with nothing in it is no name, and Fleet refuses one. */
const settled = (draft: string): string | null => {
  const name = draft.trim();
  return name === "" ? null : name;
};

export function StudioName({
  name,
  untitled,
  editable = false,
  heading = false,
  naming: asked = false,
  onNaming,
  onRename,
  saving = false,
  refused,
}: StudioNameProps) {
  // What is being typed. `null` is nobody having typed yet, which is not the
  // same as the field being shut — a control outside can open it, and then the
  // name is where the typing starts.
  const [typed, setTyped] = useState<string | null>(null);
  const field = useRef<HTMLInputElement>(null);
  const shown = name ?? untitled;
  const naming = typed !== null || asked;
  const draft = typed ?? (name ?? "");

  // The cursor goes where the name was, with what is there selected: the first
  // thing a person does to "Untitled Studio" is replace all of it.
  useEffect(() => {
    if (naming) field.current?.select();
  }, [naming]);

  function shut(): void {
    setTyped(null);
    onNaming?.(false);
  }

  // The name arriving is the write having happened — Fleet answers every write
  // with the Studio whole — so nothing here holds a second copy of it open.
  useEffect(() => {
    if (!saving && refused === undefined) shut();
  }, [name]);

  function save(): void {
    const settledName = settled(draft);
    if (settledName !== null) onRename(settledName);
  }

  function keyed(event: KeyboardEvent<HTMLInputElement>): void {
    if (event.key === "Enter") {
      event.preventDefault();
      save();
      return;
    }
    // Esc abandons the rename rather than closing the surface behind it.
    if (event.key === "Escape") {
      event.stopPropagation();
      shut();
    }
  }

  if (naming) {
    return (
      <div className="armada-studio-name armada-studio-name--naming">
        <Input
          ref={field}
          // Labelled but not captioned: the name it is replacing is right
          // there, and a caption over one field in a heading row is noise.
          aria-label="Studio name"
          value={draft}
          disabled={saving}
          invalid={refused !== undefined}
          message={refused}
          onChange={(event) => setTyped(event.target.value)}
          onKeyDown={keyed}
        />
        <div className="armada-studio-name__acts">
          <Button size="sm" variant="primary" pending={saving} disabled={settled(draft) === null} onClick={save}>
            Save
          </Button>
          <Button size="sm" variant="ghost" disabled={saving} onClick={shut}>
            Cancel
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="armada-studio-name">
      {drawn({ heading, shown, untitled: name === null })}
      {/* The Rename beside the name is the heading's. A row's is in the row's
          own split button, which opens this through `naming` — two controls for
          one act on one row is what the owner's note took away. */}
      {editable && onNaming === undefined ? (
        <Button size="sm" variant="ghost" aria-label={`Rename ${shown}`} onClick={() => setTyped(name ?? "")}>
          Rename
        </Button>
      ) : null}
    </div>
  );
}

/** The name itself: the page's heading, or the text on a row. */
function drawn({
  heading,
  shown,
  untitled,
}: {
  heading: boolean;
  shown: string;
  untitled: boolean;
}): ReactNode {
  const unnamed = untitled || undefined;
  if (heading) {
    return (
      <h2 className="armada-studio-name__heading" data-untitled={unnamed}>
        {shown}
      </h2>
    );
  }
  return (
    <span className="armada-studio-name__text" data-untitled={unnamed}>
      {shown}
    </span>
  );
}
