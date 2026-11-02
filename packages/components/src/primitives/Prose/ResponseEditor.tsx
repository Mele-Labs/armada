import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent, RefObject } from "react";
import { createPortal } from "react-dom";

import { useKept } from "../../keep";
import { Button } from "../Button/Button";
import { Input } from "../Input/Input";
import { Tooltip } from "../Tooltip/Tooltip";
import { REACTIONS, firstOne, isEmoji } from "./responses";
import type { QuickResponse } from "./responses";

const COLUMNS = 8;

export type ResponseEditorProps = {
  /** The response being edited. Absent, the editor is empty and adds one. */
  response?: QuickResponse;
  /** What the fields are kept under, so a half-typed response survives the panel closing. */
  keep: string;
  onSave: (emoji: string, name: string) => void;
  /** Present where a response is edited and can go. */
  onRemove?: () => void;
  onClose: () => void;
};

/**
 * One response's emoji and name: a grid of common reactions, a field that takes any emoji typed or pasted, and a name field.
 * **Enter saves, Esc closes.** Drawn in the hover bar's floating layer and, inline, under the settings row.
 */
export function ResponseEditor({ response, keep, onSave, onRemove, onClose }: ResponseEditorProps) {
  const base = `${keep}:${response?.id ?? "new"}`;
  const [emoji, setEmoji, clearEmoji] = useKept<string>(`${base}:emoji`, response?.emoji ?? "");
  const [name, setName, clearName] = useKept<string>(`${base}:name`, response?.name ?? "");
  const field = useRef<HTMLInputElement>(null);
  const grid = useRef<HTMLDivElement>(null);
  // A name he typed here stays; otherwise the name follows the reaction picked, so 🔥 is not left called "No to this".
  const typed = useRef(false);
  useEffect(() => {
    field.current?.focus();
    field.current?.select();
  }, []);
  const valid = isEmoji(emoji) && name.trim() !== "";
  const save = () => {
    if (!valid) return;
    onSave(emoji, name.trim());
    clearEmoji();
    clearName();
  };
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    } else if (event.key === "Enter" && (event.target as HTMLElement).tagName === "INPUT") {
      event.preventDefault();
      save();
    }
  };
  // The grid is one stop in the tab order; the arrows move within it.
  const onGridKey = (event: KeyboardEvent<HTMLDivElement>) => {
    const by = {
      ArrowLeft: -1,
      ArrowRight: 1,
      ArrowUp: -COLUMNS,
      ArrowDown: COLUMNS,
    }[event.key];
    if (by === undefined) return;
    event.preventDefault();
    const cells = [
      ...(grid.current?.querySelectorAll<HTMLElement>("button") ?? []),
    ];
    const at = cells.indexOf(document.activeElement as HTMLElement);
    cells[Math.min(cells.length - 1, Math.max(0, at + by))]?.focus();
  };
  const current = REACTIONS.findIndex((one) => one.emoji === emoji);
  return (
    <div className="armada-response-editor" data-escape-own onKeyDown={onKeyDown}>
      <div
        ref={grid}
        className="armada-response-editor__grid"
        role="group"
        aria-label="Reactions"
        onKeyDown={onGridKey}
      >
        {REACTIONS.map((one, at) => (
          <Tooltip key={one.emoji} label={one.name}>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              iconOnly
              aria-label={one.name}
              aria-pressed={emoji === one.emoji}
              tabIndex={at === (current < 0 ? 0 : current) ? 0 : -1}
              onClick={() => {
                setEmoji(one.emoji);
                if (!typed.current || name.trim() === "") setName(one.name);
              }}
            >
              <span className="armada-response-editor__emoji" aria-hidden>
                {one.emoji}
              </span>
            </Button>
          </Tooltip>
        ))}
      </div>
      <div className="armada-response-editor__fields">
        <Input
          ref={field}
          label="Emoji"
          autoComplete="off"
          value={emoji}
          invalid={emoji !== "" && !isEmoji(emoji)}
          onChange={(event) => setEmoji(firstOne(event.currentTarget.value))}
        />
        <Input
          label="Name"
          autoComplete="off"
          value={name}
          onChange={(event) => {
            typed.current = true;
            setName(event.currentTarget.value);
          }}
        />
      </div>
      <div className="armada-response-editor__acts">
        {onRemove === undefined ? null : (
          <Button type="button" variant="ghost" size="sm" onClick={onRemove}>
            Remove
          </Button>
        )}
        <span className="armada-response-editor__gap" />
        <Button type="button" variant="ghost" size="sm" onClick={onClose}>
          Cancel
        </Button>
        <Button type="button" variant="primary" size="sm" disabled={!valid} onClick={save}>
          Save
        </Button>
      </div>
    </div>
  );
}

/**
 * The editor as a floating layer over the hover bar: above it where there is room, otherwise under the line, right-aligned to the
 * bar. A press outside puts it away.
 */
export function ResponseEditorLayer({
  bar,
  line,
  label,
  onClose,
  ...editor
}: ResponseEditorProps & {
  bar: RefObject<HTMLElement | null>;
  line: RefObject<HTMLElement | null>;
  label: string;
}) {
  const panel = useRef<HTMLDivElement>(null);
  const [at, setAt] = useState<
    | {
        left: number;
        top: number;
      }
    | undefined
  >(undefined);
  useLayoutEffect(() => {
    const box = panel.current?.getBoundingClientRect();
    const near = bar.current?.getBoundingClientRect();
    const under = line.current?.getBoundingClientRect();
    if (box === undefined || near === undefined || under === undefined) return;
    const edge = 8;
    const above = near.top - box.height - 4;
    setAt({
      left: Math.max(edge, Math.min(near.right - box.width, window.innerWidth - box.width - edge)),
      top:
        above >= edge ? above : Math.min(under.bottom + 4, window.innerHeight - box.height - edge),
    });
  }, [
    bar,
    line,
  ]);
  useEffect(() => {
    const onDown = (event: MouseEvent) => {
      if (!panel.current?.contains(event.target as Node)) onClose();
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, [
    onClose,
  ]);
  return createPortal(
    <div
      ref={panel}
      className="armada-response-editor-layer"
      role="dialog"
      aria-label={label}
      style={
        at === undefined
          ? {
              visibility: "hidden",
            }
          : at
      }
    >
      <ResponseEditor {...editor} onClose={onClose} />
    </div>,
    document.body,
  );
}
