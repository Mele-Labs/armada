import { useId, useRef, useState } from "react";
import type { ClipboardEvent, DragEvent, FormEvent, KeyboardEvent } from "react";
import { Send } from "lucide-react";

import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { AUTO } from "../DispatchSettings/TierModels";
import { SketchPreview } from "../SketchPreview/SketchPreview";

/**
 * A Session's message box, with what a terminal session's has.
 *
 * **Model and effort are Dispatch's own pair** — `Select` over the models Fleet
 * lists and `low`, `medium`, `high`, each with Auto, as a Drone's settings on
 * the approval canvas draw them. **`/` opens the skills and commands**, as in
 * a terminal session, and **`@` opens the other Sessions**, choosing one tags it with
 * a chip so the agent knows to talk to it. **A picture or file is pasted or
 * dropped in**, or attached, and sits as a chip until sent. **A sketch is
 * shared** from the person's own, the ones Dispatch and Studios hold.
 */
export type ComposerFile = { id: string; name: string; src?: string };

export type SentFromComposer = {
  text: string;
  files: readonly ComposerFile[];
  sketches: readonly string[];
  mentions: readonly string[];
};

export type SessionComposerProps = {
  /** A turn is running: Send is off. */
  working: boolean;
  model: string | null;
  effort: string | null;
  models: readonly string[];
  efforts: readonly string[];
  onTune: (tuning: { model: string | null; effort: string | null }) => void;
  /** What `/` offers. */
  commands: readonly { name: string; says: string }[];
  /** What `@` offers: every other Session. */
  sessions: readonly { id: string; title: string }[];
  /** The person's own sketches, on offer to share. */
  sketches: readonly {
    id: string;
    title: string;
    drawing: {
      boxes: readonly { id: string; x: number; y: number; body: string }[];
      lines: readonly { id: string; from: string; to: string }[];
    };
  }[];
  onSend: (sent: SentFromComposer) => void;
};

type Item = { id: string; name: string; says?: string };

/** What the caret is in: a `/` at the start of the message, or an `@` at the start of a word. */
function tokenAt(text: string, caret: number): { trigger: "/" | "@"; query: string; from: number } | undefined {
  const head = text.slice(0, caret);
  const slash = /^\/(\S*)$/.exec(head);
  if (slash !== null) return { trigger: "/", query: slash[1] ?? "", from: 0 };
  const at = /(^|\s)@(\S*)$/.exec(head);
  if (at !== null) return { trigger: "@", query: at[2] ?? "", from: head.length - (at[2]?.length ?? 0) - 1 };
  return undefined;
}

export function SessionComposer({ working, model, effort, models, efforts, onTune, commands, sessions, sketches, onSend }: SessionComposerProps) {
  const [text, setText] = useState("");
  const [caret, setCaret] = useState(0);
  const [files, setFiles] = useState<ComposerFile[]>([]);
  const [shared, setShared] = useState<string[]>([]);
  const [mentioned, setMentioned] = useState<string[]>([]);
  const [active, setActive] = useState(0);
  const [offering, setOffering] = useState(false);
  const field = useRef<HTMLDivElement>(null);
  const picker = useRef<HTMLInputElement>(null);
  const counter = useRef(0);
  const listId = useId();

  const token = tokenAt(text, caret);
  const items: Item[] =
    token === undefined
      ? []
      : token.trigger === "/"
        ? commands.filter((one) => one.name.toLowerCase().includes(token.query.toLowerCase())).map((one) => ({ id: one.name, name: `/${one.name}`, says: one.says }))
        : sessions
            .filter((one) => !mentioned.includes(one.id) && one.title.toLowerCase().includes(token.query.toLowerCase()))
            .map((one) => ({ id: one.id, name: one.title }));
  const open = items.length > 0;
  const current = Math.min(active, Math.max(items.length - 1, 0));

  const add = (picked: readonly File[]) => {
    if (picked.length === 0) return;
    setFiles((was) => [
      ...was,
      ...picked.map((file) => ({
        id: `f${(counter.current += 1)}`,
        name: file.name === "" ? "Pasted file" : file.name,
        ...(file.type.startsWith("image/") ? { src: URL.createObjectURL(file) } : {}),
      })),
    ]);
  };

  const choose = (item: Item) => {
    if (token === undefined) return;
    const after = text.slice(caret);
    if (token.trigger === "/") {
      setText(`/${item.id} ${after}`);
    } else {
      setText(`${text.slice(0, token.from)}${after}`);
      setMentioned((was) => [...was, item.id]);
    }
    setActive(0);
    field.current?.querySelector("textarea")?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (!open) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setActive((current + (event.key === "ArrowDown" ? 1 : items.length - 1)) % items.length);
    } else if (event.key === "Enter" || event.key === "Tab") {
      event.preventDefault();
      choose(items[current]!);
    } else if (event.key === "Escape") {
      event.preventDefault();
      setCaret(0);
    }
  };

  const onPaste = (event: ClipboardEvent) => {
    const pasted = [...event.clipboardData.files];
    if (pasted.length === 0) return;
    event.preventDefault();
    add(pasted);
  };

  const onDrop = (event: DragEvent) => {
    event.preventDefault();
    add([...event.dataTransfer.files]);
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (working || (text.trim() === "" && files.length + shared.length + mentioned.length === 0)) return;
    onSend({ text: text.trim(), files, sketches: shared, mentions: mentioned });
    setText("");
    setCaret(0);
    setFiles([]);
    setShared([]);
    setMentioned([]);
  };

  const held = files.length + shared.length + mentioned.length > 0;
  return (
    <form className="armada-session-composer" onSubmit={submit} onDragOver={(event) => event.preventDefault()} onDrop={onDrop}>
      {!held ? null : (
        <div className="armada-session-composer__chips" role="group" aria-label="Attached">
          {files.map((one) => (
            <AttachmentChip key={one.id} filename={one.name} onRemove={() => setFiles((was) => was.filter((f) => f.id !== one.id))} />
          ))}
          {shared.map((id) => (
            <AttachmentChip key={id} filename={sketches.find((s) => s.id === id)?.title ?? id} from="Sketch" onRemove={() => setShared((was) => was.filter((s) => s !== id))} />
          ))}
          {mentioned.map((id) => (
            <AttachmentChip key={id} filename={sessions.find((s) => s.id === id)?.title ?? id} from="Session" onRemove={() => setMentioned((was) => was.filter((s) => s !== id))} />
          ))}
        </div>
      )}
      <div className="armada-session-composer__entry">
        {!open ? null : (
          <div className="armada-mention armada-session-composer__offer" id={listId} role="listbox" aria-label={token?.trigger === "/" ? "Skills and commands" : "Sessions"}>
            {items.map((item, index) => (
              <div
                key={item.id}
                role="option"
                aria-selected={index === current}
                className={index === current ? "armada-mention__row armada-mention__row--active" : "armada-mention__row"}
                onMouseEnter={() => setActive(index)}
                onMouseDown={(event) => {
                  event.preventDefault();
                  choose(item);
                }}
              >
                <span className="armada-session-composer__name">{item.name}</span>
                {item.says === undefined ? null : <span className="armada-session-composer__says">{item.says}</span>}
              </div>
            ))}
          </div>
        )}
        <div className="armada-session-composer__field" ref={field}>
          <Textarea
            aria-label="Message"
            aria-controls={open ? listId : undefined}
            rows={2}
            value={text}
            onChange={(event) => {
              setText(event.target.value);
              setCaret(event.target.selectionStart);
              setActive(0);
            }}
            onSelect={(event) => setCaret(event.currentTarget.selectionStart)}
            onKeyDown={onKeyDown}
            onPaste={onPaste}
          />
        </div>
        <Button type="submit" variant="primary" size="sm" disabled={working || (text.trim() === "" && !held)}>
          <Send size={12} strokeWidth={2} aria-hidden />
          Send
        </Button>
      </div>
      <div className="armada-session-composer__tools">
        <Select label="Model" value={model ?? ""} onChange={(event) => onTune({ model: event.target.value === "" ? null : event.target.value, effort })}>
          <option value="">{AUTO}</option>
          {models.map((one) => (
            <option key={one} value={one}>
              {one}
            </option>
          ))}
        </Select>
        <Select label="Effort" value={effort ?? ""} onChange={(event) => onTune({ model, effort: event.target.value === "" ? null : event.target.value })}>
          <option value="">{AUTO}</option>
          {efforts.map((one) => (
            <option key={one} value={one}>
              {one}
            </option>
          ))}
        </Select>
        <input
          ref={picker}
          type="file"
          multiple
          hidden
          aria-label="Files to attach"
          onChange={(event) => {
            add([...(event.target.files ?? [])]);
            event.target.value = "";
          }}
        />
        <Button type="button" variant="ghost" size="sm" onClick={() => picker.current?.click()}>
          Attach file
        </Button>
        <span className="armada-session-composer__share">
          <Button type="button" variant="ghost" size="sm" aria-expanded={offering} onClick={() => setOffering((was) => !was)}>
            Attach sketch
          </Button>
          {!offering ? null : (
            <div className="armada-mention armada-session-composer__sketches" role="listbox" aria-label="Your sketches">
              {sketches.map((one) => (
                <div
                  key={one.id}
                  role="option"
                  aria-selected={shared.includes(one.id)}
                  className="armada-mention__row armada-session-composer__sketch"
                  onMouseDown={(event) => {
                    event.preventDefault();
                    setShared((was) => (was.includes(one.id) ? was : [...was, one.id]));
                    setOffering(false);
                  }}
                >
                  <span className="armada-session-composer__thumb">
                    <SketchPreview label={`Sketch ${one.title}`} boxes={one.drawing.boxes} lines={one.drawing.lines} strokes={[]} pictures={[]} />
                  </span>
                  <span className="armada-session-composer__name">{one.title}</span>
                </div>
              ))}
            </div>
          )}
        </span>
      </div>
    </form>
  );
}
