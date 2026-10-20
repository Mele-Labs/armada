// The sheet that adds a flight control or changes one: its name, glyph and key, and what it sends or opens. A built-in that opens a layer or a
// dialog gets those three and whether it is shown; one that sends words (Update, Auto land, Compact) gets its words as well, with Reset back
// to Armada's own. A new action also has a kind, and the kinds on offer are the set's: the Job's tiles can send words, type a slash command,
// run a shell line in the Job's bay or open a page, and the tiles drawn with no Job can only open a page. The owner's ask of 18 Oct 2026.
// While a repository is open an action of the person's can be kept with it, drawn only there (`flight-controls.ts`); a built-in cannot.
// One already kept there can be committed to the repository on the open Job's branch, or shared as a package (`PieceActs`).

import { useEffect, useMemo, useRef, useState } from "react";
import { Button, Checkbox, Input, Select, Sheet, Textarea, Tooltip, forgetKept } from "@armada/components";

import { GLYPH_NAMES, GLYPHS } from "./flight-glyphs";
import { PieceActs } from "../../sharing/PieceActs";
import { KIND_GLYPH, KIND_SAID, KINDS, freshId, isWebAddress, keyProblem, type ActionKind, type FlightEntry, type FlightSet, type Tile } from "./flight-controls";

const FIELD: Record<ActionKind, { label: string; hint: string; mono: boolean }> = {
  words: { label: "Words", hint: "", mono: false },
  command: { label: "Command", hint: "/review", mono: true },
  shell: { label: "Shell line", hint: "pnpm typecheck", mono: true },
  page: { label: "Address", hint: "https://github.com/{repository}/pull/{pr}", mono: true },
};

/** Why the address cannot be opened, or nothing where it can. */
const addressProblem = (address: string): string | undefined => (address.trim() === "" || isWebAddress(address.trim()) ? undefined : "An address that starts with http:// or https://");

/** The glyphs as a group of radios: one stop for the keyboard, the arrows move and pick. */
function GlyphPicker({ value, onPick }: { value: string; onPick: (name: string) => void }) {
  const [find, setFind] = useState("");
  const shown = useMemo(() => GLYPH_NAMES.filter((one) => one.includes(find.trim().toLowerCase().replaceAll(" ", "-"))), [find]);
  const group = useRef<HTMLDivElement>(null);
  const step = (from: string, by: number) => {
    const at = shown.indexOf(from);
    const next = shown[(at + by + shown.length) % shown.length];
    if (next === undefined) return;
    onPick(next);
    window.requestAnimationFrame(() => group.current?.querySelector<HTMLElement>(`[data-glyph="${next}"]`)?.focus());
  };
  const stop = shown.includes(value) ? value : shown[0];
  return (
    <div className="flight-glyphs">
      <Input label="Find a glyph" value={find} onChange={(event) => setFind(event.target.value)} mono />
      <div
        ref={group}
        className="flight-glyphs__grid"
        role="radiogroup"
        aria-label="Glyph"
        onKeyDown={(event) => {
          const from = (event.target as HTMLElement).dataset.glyph;
          if (from === undefined) return;
          const by = event.key === "ArrowRight" || event.key === "ArrowDown" ? 1 : event.key === "ArrowLeft" || event.key === "ArrowUp" ? -1 : 0;
          if (by === 0) return;
          event.preventDefault();
          step(from, by);
        }}
      >
        {shown.map((name) => {
          const Glyph = GLYPHS[name]!;
          return (
            <Tooltip key={name} label={name} asChild>
              <button type="button" role="radio" aria-checked={name === value} aria-label={name} data-glyph={name} tabIndex={name === stop ? 0 : -1} className="flight-glyphs__one" onClick={() => onPick(name)}>
                <Glyph size={16} aria-hidden="true" />
              </button>
            </Tooltip>
          );
        })}
      </div>
    </div>
  );
}

export function FlightActionSheet({
  set,
  tile,
  tiles,
  entries,
  onSave,
  onReset,
  onRemove,
  onClose,
  only,
  kept,
}: {
  set: FlightSet;
  /** The tile being changed, or nothing for a new action. */
  tile: Tile | undefined;
  tiles: readonly Tile[];
  entries: readonly FlightEntry[];
  /** `only` is whether it is kept with the open repository. */
  onSave: (entry: FlightEntry, only: boolean) => void;
  /** Offered on a built-in with changes. */
  onReset?: () => void;
  /** Offered on an action of the owner's. */
  onRemove?: () => void;
  onClose: () => void;
  /** Offered while a repository is open and its actions were read: its name, and whether this action is kept there now. */
  only?: { name: string; on: boolean };
  /** Offered on an action already saved with the open repository: its root, and the Job open in it. */
  kept?: { root: string; sessionId: string | undefined };
}) {
  const built = tile?.builtIn;
  const kinds = KINDS[set];
  const [kind, setKind] = useState<ActionKind>(tile?.kind ?? kinds[0]!);
  const [name, setName] = useState(tile?.text ?? "");
  const [glyph, setGlyph] = useState(tile?.glyph ?? glyphName(kinds[0]!));
  const [key, setKey] = useState(tile?.entry?.key ?? "");
  const [shown, setShown] = useState(tile?.hidden !== true);
  const [words, setWords] = useState(tile?.words ?? "");
  const [onWords, setOnWords] = useState(tile?.onWords ?? "");
  const [offWords, setOffWords] = useState(tile?.offWords ?? "");
  const [touched, setTouched] = useState(tile !== undefined);
  const [there, setThere] = useState(only?.on === true);
  const nameBox = useRef<HTMLInputElement>(null);
  const keep = `flight:${set}:new:words`;
  useEffect(() => {
    const handle = window.requestAnimationFrame(() => nameBox.current?.focus());
    return () => window.cancelAnimationFrame(handle);
  }, []);
  // A new action's glyph follows its kind until a glyph is picked.
  const pickKind = (next: ActionKind) => {
    setKind(next);
    if (!touched) setGlyph(glyphName(next));
  };
  const keyed = keyProblem(key.trim(), tiles, tile?.id);
  const field = built === undefined ? FIELD[kind] : undefined;
  const address = built === undefined && kind === "page" ? addressProblem(words) : undefined;
  const ready = built !== undefined ? keyed === undefined : name.trim() !== "" && words.trim() !== "" && keyed === undefined && address === undefined && GLYPHS[glyph] !== undefined;
  const save = () => {
    if (!ready) return;
    const label = name.trim();
    const letter = key.trim().toUpperCase();
    if (built !== undefined) {
      const entry: FlightEntry = { id: built.id };
      if (label !== "" && label !== built.text) entry.name = label;
      if (glyph !== built.glyph) entry.glyph = glyph;
      if (letter !== "" && built.fixedKey !== true) entry.key = letter;
      if (!shown) entry.hidden = true;
      if (built.says !== undefined && words.trim() !== "" && words !== built.says) entry.text = words;
      if (built.on !== undefined && onWords.trim() !== "" && onWords !== built.on) entry.on_text = onWords;
      if (built.off !== undefined && offWords.trim() !== "" && offWords !== built.off) entry.off_text = offWords;
      onSave(entry, false);
      return;
    }
    const entry: FlightEntry = { id: tile?.id ?? freshId(entries, label), kind, name: label, text: words.trim() };
    if (glyph !== glyphName(kind)) entry.glyph = glyph;
    if (letter !== "") entry.key = letter;
    if (tile?.hidden === true) entry.hidden = true;
    if (tile === undefined) forgetKept(keep);
    onSave(entry, only !== undefined && there);
  };
  return (
    <Sheet open kind="flight-action" title={tile === undefined ? "Add action" : `Change ${tile.label}`} onClose={onClose} footer={<Button onClick={save} disabled={!ready}>Save</Button>}>
      <form
        className="flight-form"
        onSubmit={(event) => {
          event.preventDefault();
          save();
        }}
      >
        {tile === undefined && kinds.length > 1 ? (
          <Select label="Kind" value={kind} onChange={(event) => pickKind(event.target.value as ActionKind)}>
            {kinds.map((one) => (
              <option key={one} value={one}>
                {KIND_SAID[one]}
              </option>
            ))}
          </Select>
        ) : null}
        <Input ref={nameBox} label="Name" value={name} onChange={(event) => setName(event.target.value)} />
        <GlyphPicker
          value={glyph}
          onPick={(next) => {
            setTouched(true);
            setGlyph(next);
          }}
        />
        {built?.fixedKey === true ? null : <Input label="Key" value={key} maxLength={1} onChange={(event) => setKey(event.target.value)} invalid={keyed !== undefined} {...(keyed === undefined ? {} : { message: keyed })} />}
        {built === undefined ? null : (
          <Checkbox checked={shown} onChange={(event) => setShown(event.target.checked)}>
            Shown
          </Checkbox>
        )}
        {field === undefined ? null : field.mono ? (
          <Input label={field.label} value={words} onChange={(event) => setWords(event.target.value)} placeholder={field.hint} invalid={address !== undefined} {...(address === undefined ? {} : { message: address })} mono />
        ) : (
          <Textarea label={field.label} keep={tile === undefined ? keep : false} rows={4} value={words} onChange={(event) => setWords(event.target.value)} />
        )}
        {built?.says === undefined ? null : built.id === "compact" ? (
          <Input label="Command" value={words} onChange={(event) => setWords(event.target.value)} mono />
        ) : (
          <Textarea label="Words sent" keep={false} rows={5} value={words} onChange={(event) => setWords(event.target.value)} />
        )}
        {built?.on === undefined ? null : (
          <>
            <Textarea label="Words for on" keep={false} rows={4} value={onWords} onChange={(event) => setOnWords(event.target.value)} />
            <Textarea label="Words for off" keep={false} rows={3} value={offWords} onChange={(event) => setOffWords(event.target.value)} />
          </>
        )}
        {built !== undefined || only === undefined ? null : (
          <Checkbox checked={there} onChange={(event) => setThere(event.target.checked)}>
            Only in {only.name}
          </Checkbox>
        )}
        {kept === undefined || tile === undefined ? null : <PieceActs repository={kept.root} sessionId={kept.sessionId} piece={{ kind: "flight_controls", [set]: [tile.id] }} />}
        {onReset === undefined && onRemove === undefined ? null : (
          <div className="flight-form__acts">
            {onReset === undefined ? null : (
              <Button variant="secondary" onClick={onReset}>
                Reset
              </Button>
            )}
            {onRemove === undefined ? null : (
              <Button variant="destructive" onClick={onRemove}>
                Remove
              </Button>
            )}
          </div>
        )}
      </form>
    </Sheet>
  );
}

const glyphName = (kind: ActionKind): string => KIND_GLYPH[kind];
