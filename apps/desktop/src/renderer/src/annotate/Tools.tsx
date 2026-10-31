// The note card's tools, so the person can show what he wants and not only say it: change the
// element's size, colour, fill, spacing or words on the page, from the design tokens, and see each
// change listed with what it was. Pictures he pastes or drops on the card sit under them.

import { useEffect, useLayoutEffect, useState, type CSSProperties, type KeyboardEvent, type ReactNode } from "react";
import { ALargeSmall, PaintBucket, Palette, SquareDashed, TextCursorInput, Undo2, X } from "lucide-react";
import { Button, Textarea, Tooltip } from "@armada/components";

import { isPng, type ChangeKind, type StyleChange } from "../../../shared/annotations";
import { asHex, computed, putOn, takeOff, tokenValue, wordsOf } from "./restyle";
import { COLOURS, FILLS, SIZES, SPACES, step } from "./tokens";

const TOOLS: readonly { kind: ChangeKind; label: string; icon: ReactNode }[] = [
  { kind: "size", label: "Size", icon: <ALargeSmall size={14} strokeWidth={2} aria-hidden /> },
  { kind: "colour", label: "Colour", icon: <Palette size={14} strokeWidth={2} aria-hidden /> },
  { kind: "fill", label: "Fill", icon: <PaintBucket size={14} strokeWidth={2} aria-hidden /> },
  { kind: "spacing", label: "Spacing", icon: <SquareDashed size={14} strokeWidth={2} aria-hidden /> },
  { kind: "text", label: "Text", icon: <TextCursorInput size={14} strokeWidth={2} aria-hidden /> },
];

/** Left and right move between the buttons of a row, so a row is one stop on Tab and its buttons are a key apart. */
function arrows(event: KeyboardEvent<HTMLElement>): void {
  if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;
  const buttons = [...event.currentTarget.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
  const at = buttons.indexOf(document.activeElement as HTMLButtonElement);
  if (at === -1) return;
  event.preventDefault();
  buttons[(at + (event.key === "ArrowRight" ? 1 : -1) + buttons.length) % buttons.length]?.focus();
}

/** `changes` with `next` in place of the change to the same property, or after them where there is none. */
function withChange(changes: readonly StyleChange[], next: StyleChange): StyleChange[] {
  return changes.some((one) => one.property === next.property) ? changes.map((one) => (one.property === next.property ? next : one)) : [...changes, next];
}

/**
 * `changes` put on `element` while the card is up, and taken off when it goes. A layout effect, so the
 * first frame drawn with the card already shows them.
 */
function useRestyled(element: Element | null, changes: readonly StyleChange[], id: string): void {
  useLayoutEffect(() => {
    if (element === null) return undefined;
    putOn(element, changes, id);
    return () => takeOff(element);
  }, [element, changes, id]);
}

/** `id` is the note's, which names the rule its Size and Colour are written in. */
export function Tools({ id, element, changes, onChanges, keep }: { id: string; element: Element | null; changes: readonly StyleChange[]; onChanges: (next: StyleChange[]) => void; keep: string }) {
  const [tool, setTool] = useState<ChangeKind | null>(null);
  useRestyled(element, changes, id);
  const words = element === null ? null : wordsOf(element);
  // Words cleared to nothing are still his to type back.
  const hasWords = words !== null || changes.some((one) => one.kind === "text");

  /** A token picked for `property`: what it was is read before this property is changed, and kept from the first change to it. */
  function pick(kind: ChangeKind, property: string, token: string): void {
    if (element === null) return;
    const was = changes.find((one) => one.property === property);
    // Colours are read as the page computes them, and written as the tokens are.
    const read = kind === "colour" || kind === "fill" ? asHex : (value: string) => value;
    const before = was?.before ?? read(computed(element, property));
    onChanges(withChange(changes, { kind, property, before, after: read(tokenValue(element, token)), token }));
  }

  function typed(text: string): void {
    const was = changes.find((one) => one.kind === "text");
    const before = was?.before ?? words?.nodeValue ?? "";
    if (text === before) {
      onChanges(changes.filter((one) => one.kind !== "text"));
      return;
    }
    onChanges(withChange(changes, { kind: "text", property: "text", before, after: text }));
  }

  const tokenOf = (property: string): string | undefined => changes.find((one) => one.property === property)?.token;
  const textNow = changes.find((one) => one.kind === "text")?.after ?? words?.nodeValue ?? "";

  return (
    <div className="armada-annotate__tools">
      <div className="armada-annotate__toolbar" role="toolbar" aria-label="Tools" onKeyDown={arrows}>
        {TOOLS.map(({ kind, label, icon }) => {
          const off = element === null || (kind === "text" && !hasWords);
          return (
            <Tooltip key={kind} label={off && element !== null ? `${label}: no words on it` : label} asChild>
              <Button variant="ghost" size="sm" iconOnly aria-label={label} aria-pressed={tool === kind} disabled={off} onClick={() => setTool(tool === kind ? null : kind)}>
                {icon}
              </Button>
            </Tooltip>
          );
        })}
      </div>
      {element !== null && tool === "size" && <Scale label="Size" tokens={SIZES} current={tokenOf("font-size")} element={element} onPick={(token) => pick("size", "font-size", token)} />}
      {element !== null && tool === "colour" && <Swatches label="Colour" tokens={COLOURS} current={tokenOf("color")} element={element} onPick={(token) => pick("colour", "color", token)} />}
      {element !== null && tool === "fill" && <Swatches label="Fill" tokens={FILLS} current={tokenOf("background-color")} element={element} onPick={(token) => pick("fill", "background-color", token)} />}
      {element !== null && tool === "spacing" && (
        <>
          <Scale label="Padding" tokens={SPACES} current={tokenOf("padding")} element={element} onPick={(token) => pick("spacing", "padding", token)} />
          <Scale label="Gap" tokens={SPACES} current={tokenOf("gap")} element={element} onPick={(token) => pick("spacing", "gap", token)} />
        </>
      )}
      {element !== null && tool === "text" && hasWords && <Textarea label="Text" rows={2} keep={keep} value={textNow} onChange={(event) => typed(event.target.value)} />}
      <Changes changes={changes} onChanges={onChanges} />
    </div>
  );
}

/** A scale's steps as buttons reading the step, each with its token and value as its tooltip. */
function Scale({ label, tokens, current, element, onPick }: { label: string; tokens: readonly string[]; current: string | undefined; element: Element; onPick: (token: string) => void }) {
  return (
    <div className="armada-annotate__picker" role="group" aria-label={label} onKeyDown={arrows}>
      <span className="armada-annotate__picker-label">{label}</span>
      <div className="armada-annotate__picker-steps">
        {tokens.map((token) => (
          <Tooltip key={token} label={`${token} ${tokenValue(element, token)}`} asChild>
            <Button variant="ghost" size="sm" aria-label={token} aria-pressed={current === token} onClick={() => onPick(token)}>
              {step(token)}
            </Button>
          </Tooltip>
        ))}
      </div>
    </div>
  );
}

/** Colour tokens as swatches, each with its token and value as its tooltip. */
function Swatches({ label, tokens, current, element, onPick }: { label: string; tokens: readonly string[]; current: string | undefined; element: Element; onPick: (token: string) => void }) {
  return (
    <div className="armada-annotate__picker" role="group" aria-label={label} onKeyDown={arrows}>
      <span className="armada-annotate__picker-label">{label}</span>
      <div className="armada-annotate__picker-steps">
        {tokens.map((token) => (
          <Tooltip key={token} label={`${token} ${tokenValue(element, token)}`} asChild>
            <button
              type="button"
              className="armada-annotate__swatch"
              aria-label={token}
              aria-pressed={current === token}
              style={{ "--annotate-swatch": `var(${token})` } as CSSProperties}
              onClick={() => onPick(token)}
            />
          </Tooltip>
        ))}
      </div>
    </div>
  );
}

const shown = (change: StyleChange, value: string): string => (change.kind === "text" ? value : value === "" ? "none" : value);

/** What a change's row calls its property: the tool's name, and Padding and Gap apart. */
const LABELS: Record<string, string> = { "font-size": "Size", color: "Colour", "background-color": "Fill", padding: "Padding", gap: "Gap", text: "Text" };
const labelOf = (change: StyleChange): string => LABELS[change.property] ?? change.property;

/** Each change, what it was and what it is, with its token; undone one at a time or all at once. */
function Changes({ changes, onChanges }: { changes: readonly StyleChange[]; onChanges: (next: StyleChange[]) => void }) {
  if (changes.length === 0) return null;
  return (
    <>
      <ul className="armada-annotate__changes" aria-label="Changes">
        {changes.map((change) => (
          <li key={change.property} className="armada-annotate__change">
            <Tooltip label={change.property} asChild>
              <span className="armada-annotate__change-property">{labelOf(change)}</span>
            </Tooltip>
            <span className="armada-annotate__change-values">
              {shown(change, change.before)} → {shown(change, change.after)}
            </span>
            <span className="armada-annotate__change-token">{change.token ?? ""}</span>
            <Tooltip label="Undo" asChild>
              <Button variant="ghost" size="sm" iconOnly aria-label={`Undo ${labelOf(change)}`} onClick={() => onChanges(changes.filter((one) => one.property !== change.property))}>
                <Undo2 size={14} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          </li>
        ))}
      </ul>
      <div className="armada-annotate__actions">
        <Button variant="ghost" size="sm" onClick={() => onChanges([])}>
          Undo all
        </Button>
      </div>
    </>
  );
}

/** A picture pasted or dropped, as a PNG: one already is kept as it came, any other image is drawn again as one. Null where it is not an image. */
export async function asPng(file: Blob): Promise<ArrayBuffer | null> {
  const bytes = await file.arrayBuffer();
  if (isPng(new Uint8Array(bytes))) return bytes;
  const bitmap = await createImageBitmap(file).catch(() => null);
  if (bitmap === null) return null;
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  canvas.getContext("2d")?.drawImage(bitmap, 0, 0);
  bitmap.close();
  const png = await new Promise<Blob | null>((done) => canvas.toBlob(done, "image/png"));
  if (png === null) return null;
  const drawn = await png.arrayBuffer();
  return isPng(new Uint8Array(drawn)) ? drawn : null;
}

/** The image files a paste or a drop carries. */
export function imagesIn(data: DataTransfer | null): File[] {
  return [...(data?.files ?? [])].filter((file) => file.type.startsWith("image/"));
}

/** The note's example pictures as thumbnails, each with a press that takes it off. */
export function Examples({ pictures, onRemove }: { pictures: readonly { name: string; png: ArrayBuffer | undefined }[]; onRemove: (name: string) => void }) {
  if (pictures.length === 0) return null;
  return (
    <ul className="armada-annotate__examples" aria-label="Pictures">
      {pictures.map(({ name, png }) => (
        <li key={name} className="armada-annotate__example">
          <Thumbnail name={name} png={png} />
          <Tooltip label="Remove picture" asChild>
            <Button variant="ghost" size="sm" iconOnly aria-label={`Remove ${name}`} onClick={() => onRemove(name)}>
              <X size={14} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </li>
      ))}
    </ul>
  );
}

function Thumbnail({ name, png }: { name: string; png: ArrayBuffer | undefined }) {
  // Made and let go in one effect, so a remount, as StrictMode makes, never draws an address already let go.
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    if (png === undefined) return undefined;
    const made = URL.createObjectURL(new Blob([png], { type: "image/png" }));
    setSrc(made);
    return () => {
      URL.revokeObjectURL(made);
      setSrc(null);
    };
  }, [png]);
  return src === null ? <span className="armada-annotate__thumb" /> : <img className="armada-annotate__thumb" src={src} alt={name} />;
}
