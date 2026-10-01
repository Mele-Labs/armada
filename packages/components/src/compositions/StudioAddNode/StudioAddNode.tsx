import { useEffect, useState } from "react";
import type { KeyboardEvent } from "react";

import { keyFor } from "../../actions";
import { Button } from "../../primitives/Button/Button";
import { DropdownMenu } from "../../primitives/DropdownMenu/DropdownMenu";
import { Input } from "../../primitives/Input/Input";
import { Kbd, KbdChord } from "../../primitives/Kbd/Kbd";
import { Textarea } from "../../primitives/Textarea/Textarea";

/**
 * Add a node — the `+ Node` control on an open Studio, and the field behind
 * each of the two kinds a person writes onto one by hand. `#1364`.
 *
 * **Two kinds have a field**: a Note typed and a Link pasted. A Sketch is on the
 * menu and drawn on the pad (1 Oct 2026); a File and a Picture are only ever
 * pasted onto the board. A Finding comes from a scout, a Run from a run, a
 * Cluster from promotion — and Fleet refuses those from Bridge by name.
 *
 * **Which kind is being written is the caller's**, because the keys are: `N`,
 * `V` and `S` are live on the open Studio, and a menu holding its own state
 * would answer a press and the key would not.
 */

export type StudioNodeByHandKind = "note" | "link" | "sketch";

/** The kinds written in a field here. A Sketch is drawn on the pad instead. */
export type StudioNodeWrittenKind = Exclude<StudioNodeByHandKind, "sketch">;

export type StudioNodeByHand =
  | { kind: "note"; said: string }
  /** `said` is the line beside the address, absent where none was typed. `#1378`. */
  | { kind: "link"; address: string; said?: string }
  /** A path, pasted onto the board. **No field writes one**, so it is no `StudioNodeByHandKind`. */
  | { kind: "file"; path: string };

export type StudioAddNodeProps = {
  /** The kind being written, or `null` for the menu alone. */
  adding: StudioNodeWrittenKind | null;
  onAdding: (kind: StudioNodeByHandKind | null) => void;
  /** What was written. Never called with a blank field. */
  onAdd: (node: StudioNodeByHand) => void;
  /**
   * What *Read it in* does with a pasted address — `#1293`. A function is the
   * act; a string is why it is not offered, drawn beside the choice.
   *
   * **The choice is drawn refused rather than left out**, because one choice
   * with no mention of the other reads as the whole offer.
   */
  readIn: ((node: StudioNodeByHand) => void) | string;
  /** Out to Fleet: the field waits rather than taking a second press. */
  saving?: boolean;
  /** Why the last node did not land, or absent. */
  refused?: string;
  /** A Studio reopened read-only, or a window with no connection. */
  disabled?: boolean;
  /**
   * The field inside the node it makes — the owner's notes of 1 Oct 2026, which
   * asked that a kind be put down on the board rather than written in a panel.
   * The node's head names the kind, so the label is read aloud and not drawn.
   * **No Cancel, and no press off the node does anything**: Esc is the way out,
   * since a draft that went with a stray press was what he called too
   * sensitive.
   */
  inPlace?: boolean;
  /** What a send would carry now, or `null` while blank — for a caller that sends it on another press. */
  onWritten?: (node: StudioNodeByHand | null) => void;
};

/** Each kind's act in the registry, which is the one place a binding is written. */
const ACT: Readonly<Record<StudioNodeByHandKind, string>> = {
  note: "add_note",
  link: "add_link",
  sketch: "add_sketch",
};

/** What the field is called, what it asks for, and how tall it is drawn. */
const FIELD: Readonly<Record<StudioNodeWrittenKind, { label: string; asks: string; rows: number }>> = {
  note: { label: "Note", asks: "What you noticed", rows: 3 },
  link: { label: "Link", asks: "A board, document, issue, page or session, as its address", rows: 0 },
};

/** What the menu calls each kind, the Sketch it opens the pad on included. */
const NAMED: Readonly<Record<StudioNodeByHandKind, string>> = {
  note: "Note",
  link: "Link",
  sketch: "Sketch",
};

const KINDS: readonly StudioNodeByHandKind[] = ["note", "link", "sketch"];

/**
 * What reading an address in produces, said before anybody presses it —
 * `docs/concepts/studio.md`, *Promotion*.
 */
const READING_IN_MAKES =
  "Reading it in makes Notes and Contradictions out of what the page says, each with an edge back " +
  "to the Link. The Link stays where it is.";

/** What a person wrote, as the node it makes. Blank is nothing, and sends nothing. */
function written(kind: StudioNodeWrittenKind, draft: string, line: string): StudioNodeByHand | null {
  const said = draft.trim();
  if (said === "") return null;
  if (kind === "note") return { kind, said };
  // A Link is its address whatever is typed beside it, so a blank line is left
  // out rather than sent — one shape for a Link nobody wrote a line on.
  return line.trim() === "" ? { kind, address: said } : { kind, address: said, said: line.trim() };
}

export function StudioAddNode({
  adding,
  onAdding,
  onAdd,
  readIn,
  saving = false,
  refused,
  disabled = false,
  inPlace = false,
  onWritten,
}: StudioAddNodeProps) {
  const [draft, setDraft] = useState("");
  const [line, setLine] = useState("");

  // A new kind is a new field: what was half-typed for a Note is not a Link.
  // The field itself is keyed on the kind, so it mounts afresh and takes focus.
  useEffect(() => {
    setDraft("");
    setLine("");
  }, [adding]);

  useEffect(() => {
    if (adding !== null) onWritten?.(written(adding, draft, line));
  }, [adding, draft, line, onWritten]);

  if (adding === null) {
    return (
      <DropdownMenu
        triggerLabel="+ Node"
        disabled={disabled}
        entries={KINDS.map((kind) => ({
          kind: "item" as const,
          id: kind,
          label: NAMED[kind],
          shortcut: keyFor(ACT[kind]),
        }))}
        onSelect={(id) => onAdding(id as StudioNodeByHandKind)}
      />
    );
  }

  const { label, asks, rows } = FIELD[adding];
  const node = written(adding, draft, line);

  function add(): void {
    if (node !== null) onAdd(node);
  }

  // Enter sends a line; a field a person writes prose in takes ⌘Enter, so a
  // paragraph break does not send the note. Esc abandons it either way.
  function keyed(event: KeyboardEvent<HTMLElement>): void {
    if (event.key === "Escape") {
      event.stopPropagation();
      onAdding(null);
      return;
    }
    if (event.key !== "Enter") return;
    if (adding !== "link" && !(event.metaKey || event.ctrlKey)) return;
    event.preventDefault();
    add();
  }

  return (
    <div className="armada-studio-add-node">
      {adding === "link" ? (
        <Input
          key={adding}
          autoFocus
          {...(inPlace ? { "aria-label": label } : { label })}
          placeholder={asks}
          value={draft}
          mono
          disabled={saving}
          invalid={refused !== undefined}
          message={refused}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={keyed}
        />
      ) : (
        <Textarea
          key={adding}
          autoFocus
          {...(inPlace ? { "aria-label": label } : { label })}
          placeholder={asks}
          rows={rows}
          value={draft}
          disabled={saving}
          invalid={refused !== undefined}
          message={refused}
          onChange={(event) => setDraft(event.target.value)}
          onKeyDown={keyed}
        />
      )}
      {adding === "link" && draft.trim() !== "" ? (
        <PastedOffer line={line} onLine={setLine} onKeyDown={keyed} readIn={readIn} />
      ) : null}
      <div className="armada-studio-add-node__acts">
        <Button size="sm" variant="primary" pending={saving} disabled={node === null} onClick={add}>
          {adding === "link" ? "Keep the link" : `Add ${label.toLowerCase()}`}
        </Button>
        {/* The two choices sit beside each other: one offer of two, rather
            than an act and a suggestion in two places. */}
        {adding === "link" ? (
          <Button
            size="sm"
            disabled={typeof readIn !== "function" || saving || node === null}
            onClick={() => {
              if (typeof readIn === "function" && node !== null) readIn(node);
            }}
          >
            Read it in
          </Button>
        ) : null}
        {inPlace ? null : (
          <Button size="sm" variant="ghost" disabled={saving} onClick={() => onAdding(null)}>
            Cancel
          </Button>
        )}
        {adding === "link" ? <Kbd>Enter</Kbd> : <KbdChord keys={["⌘", "Enter"]} />}
      </div>
    </div>
  );
}

type PastedOfferProps = {
  line: string;
  onLine: (line: string) => void;
  onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
  readIn: ((node: StudioNodeByHand) => void) | string;
};

/**
 * What to do with an address somebody pasted — `#1378`.
 *
 * **Here and not in a dialog**: pasting, deciding and saying why are one act,
 * and a framed layer would put the board being placed on behind a scrim.
 *
 * **The line is optional and the address is not.** A Link never stops being
 * its address; the line is the person's own, so a Studio holding several reads
 * as sentences rather than as a list of URLs.
 */
function PastedOffer({ line, onLine, onKeyDown, readIn }: PastedOfferProps) {
  return (
    <div className="armada-studio-add-node__offer" role="group" aria-label="What to do with this address">
      <Input
        label="Your line"
        placeholder="Why you kept it"
        value={line}
        onChange={(event) => onLine(event.target.value)}
        onKeyDown={onKeyDown}
      />
      <p className="armada-studio-add-node__says">
        {typeof readIn === "function" ? READING_IN_MAKES : readIn}
      </p>
    </div>
  );
}
