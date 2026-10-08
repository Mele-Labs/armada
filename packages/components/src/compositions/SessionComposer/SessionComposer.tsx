import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { ClipboardEvent, DragEvent, FormEvent, KeyboardEvent, ReactNode } from "react";
import { Cpu, Paperclip, PencilRuler, Send, Shield, Zap } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { useKept } from "../../keep";
import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { TAG_GLYPH } from "./InlineTag";
import type { ComposerTag } from "./InlineTag";
import { beforeCaret, caretToEnd, CHIP, readDom, sentText, tagNode, writeDom } from "./message-dom";

/**
 * A Session's message box, with what a terminal session's has, in **one box
 * and one slim row under the typing area**. The row holds the permission mode,
 * Dispatch's own model and effort pair, an act to attach a file and one to draw
 * a sketch, then what is waiting to be sent in a single row that scrolls
 * sideways, so nothing waiting ever takes room from the typing area.
 *
 * **`/` opens the skills and commands, and `@` opens the other Sessions**,
 * choosing one writes it into the message as a chip, at the caret, so the agent knows to talk to it. **A
 * picture or file is pasted or dropped in**, or attached. **A sketch is drawn
 * on the pad Dispatch draws on**, which the host opens, and comes back as a
 * chip.
 */
export type ComposerFile = {
  id: string;
  name: string;
  src?: string;
  /** What was picked, so a host that sends it has its bytes. A host that only draws a chip ignores it. */
  file?: File;
};

export type { ComposerTag };

export type SentFromComposer = {
  text: string;
  files: readonly ComposerFile[];
  tags: readonly ComposerTag[];
};

/** The kinds in the order `@` groups them, each named for the group's label. */
const TAG_KINDS: { kind: ComposerTag["kind"]; label: string }[] = [
  { kind: "job", label: "Jobs" },
  { kind: "pull_request", label: "Pull requests" },
  { kind: "branch", label: "Branches" },
  { kind: "session", label: "Sessions" },
];

/** The permission modes, as the terminal's: ask, auto, accept edits, plan. */
export type ComposerMode = "ask" | "auto" | "accept_edits" | "plan";

const MODES: { id: ComposerMode; label: string; says: string }[] = [
  { id: "ask", label: "Ask", says: "Asks before it edits or runs anything" },
  { id: "auto", label: "Auto", says: "Runs on its own, and asks before a push or anything destructive" },
  { id: "accept_edits", label: "Accept edits", says: "Edits files without asking, and asks before running a command" },
  { id: "plan", label: "Plan", says: "Reads and plans, and changes nothing" },
];

export type SessionComposerProps = {
  /** A turn is running: Send is off. */
  working: boolean;
  mode: ComposerMode;
  onMode: (mode: ComposerMode) => void;
  model: string | null;
  effort: string | null;
  models: readonly string[];
  efforts: readonly string[];
  onTune: (tuning: { model: string | null; effort: string | null }) => void;
  /** What `/` offers. */
  commands: readonly { name: string; says: string }[];
  /** What `@` offers, grouped by kind: other Sessions, Jobs, pull requests and branches. */
  taggable: readonly ComposerTag[];
  /**
   * Tags that arrive from outside the box (a Job opened in a Session): each is written in at the
   * end of the message, and `onTags([])` says it was taken.
   */
  tags: readonly ComposerTag[];
  onTags: (tags: readonly ComposerTag[]) => void;
  /** Below the layout breakpoint: the selects are a glyph and a value, named on hover. */
  compact?: boolean;
  /** Sketches drawn for this message and waiting to go. */
  drawn: readonly { id: string; title: string }[];
  /** Opens the pad. */
  onDraw: () => void;
  onRemoveDrawn: (id: string) => void;
  onSend: (sent: SentFromComposer) => void;
  /** The permission mode is shown and not set: a session in a terminal holds it, and nothing outside can switch it. */
  modeLocked?: boolean;
  /** A terminal's mode is not drawn until its mod has reported one. */
  modeHidden?: boolean;
  /**
   * What the message is about, the Session's id. Its text, its tags and its files are kept under it, so leaving
   * the Session and coming back, or reloading Bridge, finds them (the text; a file's bytes do not
   * survive a reload). Cleared by a send. Without it nothing is kept.
   */
  draftKey?: string;
};

/** The message: its text holds a CHIP where each tag stands, and `tags` lists them in order. */
type Draft = { text: string; tags: ComposerTag[]; files: ComposerFile[] };
const BLANK: Draft = { text: "", tags: [], files: [] };

/** The commands a `/` query keeps: names that start with it first, then names that hold it. */
function matching<T extends { name: string }>(commands: readonly T[], query: string): T[] {
  const wanted = query.toLowerCase();
  const starts = commands.filter((one) => one.name.toLowerCase().startsWith(wanted));
  const holds = commands.filter((one) => !one.name.toLowerCase().startsWith(wanted) && one.name.toLowerCase().includes(wanted));
  return [...starts, ...holds];
}

type Item = { id: string; name: string; says?: string; tag?: ComposerTag };

/** What the caret is in: a `/` at the start of the message, or an `@` at the start of a word. */
function tokenAt(text: string, caret: number): { trigger: "/" | "@"; query: string; from: number } | undefined {
  const head = text.slice(0, caret);
  const slash = /^\/(\S*)$/.exec(head);
  if (slash !== null) return { trigger: "/", query: slash[1] ?? "", from: 0 };
  const at = /(^|\s)@(\S*)$/.exec(head);
  if (at !== null) return { trigger: "@", query: at[2] ?? "", from: head.length - (at[2]?.length ?? 0) - 1 };
  return undefined;
}

export function SessionComposer({
  working,
  mode,
  onMode,
  model,
  effort,
  models,
  efforts,
  onTune,
  commands,
  taggable,
  tags,
  onTags,
  compact = false,
  drawn,
  onDraw,
  onRemoveDrawn,
  onSend,
  modeLocked = false,
  modeHidden = false,
  draftKey,
}: SessionComposerProps) {
  const [draft, setDraft, forget] = useKept<Draft>(draftKey === undefined ? undefined : `session:${draftKey}`, BLANK, {
    stored: (one) => ({ text: one.text, tags: one.tags }),
    revive: (raw) => {
      const one = raw as Partial<Draft> | null;
      return { text: typeof one?.text === "string" ? one.text : "", tags: Array.isArray(one?.tags) ? one.tags : [], files: [] };
    },
  });
  const { text, files } = draft;
  const setFiles = (next: (was: ComposerFile[]) => ComposerFile[]) => setDraft((was) => ({ ...was, files: next(was.files) }));
  const editor = useRef<HTMLDivElement>(null);
  const [head, setHead] = useState("");
  const [active, setActive] = useState(0);
  const picker = useRef<HTMLInputElement>(null);
  const counter = useRef(0);
  const listId = useId();

  const token = tokenAt(head, head.length);
  const items: Item[] =
    token === undefined
      ? []
      : token.trigger === "/"
        ? matching(commands, token.query).map((one) => ({ id: one.name, name: `/${one.name}`, ...(one.says === "" ? {} : { says: one.says }) }))
        : TAG_KINDS.flatMap(({ kind }) =>
            taggable
              .filter((one) => one.kind === kind && !draft.tags.some((had) => had.kind === kind && had.id === one.id) && one.title.toLowerCase().includes(token.query.toLowerCase()))
              .map((one) => ({ id: `${kind}${one.id}`, name: one.title, tag: one })),
          );
  const open = items.length > 0;
  const current = Math.min(active, Math.max(items.length - 1, 0));

  // The box is drawn from the draft, and only when the box does not already say what the draft does:
  // what is typed in it is read out, never written back over the caret.
  useLayoutEffect(() => {
    const box = editor.current;
    if (box === null) return;
    const now = readDom(box);
    if (now.text === draft.text && now.tags.map((one) => one.id).join() === draft.tags.map((one) => one.id).join()) return;
    writeDom(box, draft.text, draft.tags);
    if (document.activeElement === box) caretToEnd(box);
  }, [draft.text, draft.tags]);

  // The box takes the most of half the panel it sits in, then scrolls.
  useLayoutEffect(() => {
    const box = editor.current;
    if (box === null) return;
    const bound = box.closest("form")?.parentElement;
    box.style.maxHeight = bound ? `${bound.clientHeight / 2}px` : "";
  }, [draft.text]);

  // The caret moves by key, by press and by what is typed; the typeahead follows it.
  useEffect(() => {
    const follow = () => {
      const box = editor.current;
      if (box !== null && box.contains(window.getSelection()?.anchorNode ?? null)) setHead(beforeCaret(box)?.head ?? "");
    };
    document.addEventListener("selectionchange", follow);
    return () => document.removeEventListener("selectionchange", follow);
  }, []);

  // Tags that arrive from outside are written in at the end.
  useEffect(() => {
    if (tags.length === 0) return;
    setDraft((was) => {
      const fresh = tags.filter((one) => !was.tags.some((had) => had.kind === one.kind && had.id === one.id));
      return fresh.length === 0 ? was : { ...was, text: `${was.text}${was.text === "" || was.text.endsWith(" ") ? "" : " "}${fresh.map(() => CHIP).join(" ")} `, tags: [...was.tags, ...fresh] };
    });
    onTags([]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [tags]);

  const read = () => {
    const box = editor.current;
    if (box === null) return;
    const now = readDom(box);
    // The box carries a tag's kind, id and title; the rest of what the host gave it (a Job's number, its state) is looked up.
    setDraft((was) => ({
      ...was,
      text: now.text,
      tags: now.tags.map((one) => [...was.tags, ...taggable].find((known) => known.kind === one.kind && known.id === one.id) ?? one),
    }));
    setActive(0);
  };

  const add = (picked: readonly File[]) => {
    if (picked.length === 0) return;
    setFiles((was) => [
      ...was,
      ...picked.map((file) => ({
        id: `f${(counter.current += 1)}`,
        name: file.name === "" ? "Pasted file" : file.name,
        file,
        ...(file.type.startsWith("image/") ? { src: URL.createObjectURL(file) } : {}),
      })),
    ]);
  };

  const choose = (item: Item) => {
    const box = editor.current;
    const at = box === null ? undefined : beforeCaret(box);
    if (token === undefined || box === null || at?.node == null) return;
    const { node, offset } = at;
    const selection = window.getSelection();
    const place = (where: Text, to: number) => {
      const range = document.createRange();
      range.setStart(where, to);
      range.collapse(true);
      selection?.removeAllRanges();
      selection?.addRange(range);
    };
    if (token.trigger === "/") {
      node.data = `/${item.id} ${node.data.slice(offset)}`;
      place(node, item.id.length + 2);
    } else if (item.tag !== undefined) {
      const from = offset - token.query.length - 1;
      node.data = node.data.slice(0, from) + node.data.slice(offset);
      const after = node.splitText(from);
      node.parentNode!.insertBefore(tagNode(item.tag), after);
      if (!after.data.startsWith(" ")) after.data = ` ${after.data}`;
      place(after, 1);
    }
    read();
    setHead("");
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Enter" && event.shiftKey && !event.nativeEvent.isComposing && !open) {
      event.preventDefault();
      document.execCommand("insertLineBreak");
      return;
    }
    if (!open) {
      // Enter sends and Shift+Enter breaks the line; a key that confirms an IME candidate is neither.
      if (event.key === "Enter" && !event.shiftKey && !event.nativeEvent.isComposing) {
        event.preventDefault();
        send();
      }
      return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setActive((current + (event.key === "ArrowDown" ? 1 : items.length - 1)) % items.length);
    } else if (event.key === "Enter" || event.key === "Tab") {
      event.preventDefault();
      choose(items[current]!);
    } else if (event.key === "Escape") {
      event.preventDefault();
      setHead("");
    }
  };

  const onPaste = (event: ClipboardEvent) => {
    event.preventDefault();
    const pasted = [...event.clipboardData.files];
    if (pasted.length > 0) add(pasted);
    else document.execCommand("insertText", false, event.clipboardData.getData("text/plain"));
  };

  const onDrop = (event: DragEvent) => {
    event.preventDefault();
    add([...event.dataTransfer.files]);
  };

  const held = files.length + drawn.length > 0;

  const send = () => {
    if (working || (text.trim() === "" && !held)) return;
    onSend({ text: sentText(text, draft.tags).trim(), files, tags: draft.tags });
    forget();
    setHead("");
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    send();
  };

  const said = MODES.find((one) => one.id === mode)!.says;
  return (
    <form className="armada-session-composer" onSubmit={submit} onDragOver={(event) => event.preventDefault()} onDrop={onDrop}>
      {!open ? null : (
        <div className="armada-mention armada-session-composer__offer" id={listId} role="listbox" aria-label={token?.trigger === "/" ? "Skills and commands" : "Tag a Session, Job, pull request or branch"}>
          {token?.trigger === "/"
            ? items.map((item, index) => <Row key={item.id} item={item} on={index === current} onHover={() => setActive(index)} onChoose={() => choose(item)} />)
            : TAG_KINDS.map(({ kind, label }) => {
                const rows = items.filter((one) => one.tag?.kind === kind);
                if (rows.length === 0) return null;
                return (
                  <div key={kind} role="group" aria-label={label}>
                    <p className="armada-session-composer__group" aria-hidden>
                      {label}
                    </p>
                    {rows.map((item) => (
                      <Row key={item.id} item={item} on={items.indexOf(item) === current} onHover={() => setActive(items.indexOf(item))} onChoose={() => choose(item)} />
                    ))}
                  </div>
                );
              })}
        </div>
      )}
      <div className="armada-session-composer__field">
        <div
          ref={editor}
          role="textbox"
          aria-label="Message"
          aria-multiline
          aria-controls={open ? listId : undefined}
          contentEditable
          suppressContentEditableWarning
          className="armada-textarea armada-session-composer__editor"
          onInput={read}
          onKeyDown={onKeyDown}
          onPaste={onPaste}
        />
      </div>
      <div className="armada-session-composer__bar">
        {modeHidden ? null : (
          <Pick Glyph={Shield} compact={compact} label={modeLocked ? `${said} The terminal holds it.` : said}>
            <Select aria-label="Permission mode" disabled={modeLocked} value={mode} onChange={(event) => onMode(event.target.value as ComposerMode)}>
              {MODES.map((one) => (
                <option key={one.id} value={one.id}>
                  {one.label}
                </option>
              ))}
            </Select>
          </Pick>
        )}
        <Pick Glyph={Cpu} compact={compact} label="The model the next turn runs on">
          <Select aria-label="Model" value={model ?? ""} onChange={(event) => onTune({ model: event.target.value === "" ? null : event.target.value, effort })}>
            <option value="">{compact ? "Auto" : "Model"}</option>
            {models.map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
        </Pick>
        <Pick Glyph={Zap} compact={compact} label="The effort the next turn takes">
          <Select aria-label="Effort" value={effort ?? ""} onChange={(event) => onTune({ model, effort: event.target.value === "" ? null : event.target.value })}>
            <option value="">{compact ? "Auto" : "Effort"}</option>
            {efforts.map((one) => (
              <option key={one} value={one}>
                {one}
              </option>
            ))}
          </Select>
        </Pick>
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
        <Tooltip label="Attach a file or a picture">
          <Button type="button" variant="ghost" size="sm" aria-label="Attach file" onClick={() => picker.current?.click()}>
            <Paperclip size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
        <Tooltip label="Draw a sketch to send with the message">
          <Button type="button" variant="ghost" size="sm" aria-label="Draw sketch" onClick={onDraw}>
            <PencilRuler size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
        <div className="armada-session-composer__chips" role="group" aria-label="Attached">
          {files.map((one) => (
            <AttachmentChip key={one.id} filename={one.name} onRemove={() => setFiles((was) => was.filter((f) => f.id !== one.id))} />
          ))}
          {drawn.map((one) => (
            <AttachmentChip key={one.id} filename={one.title} from="Sketch" onRemove={() => onRemoveDrawn(one.id)} />
          ))}
        </div>
        <Button type="submit" variant="primary" size="sm" disabled={working || (text.trim() === "" && !held)}>
          <Send size={12} strokeWidth={2} aria-hidden />
          Send
        </Button>
      </div>
    </form>
  );
}

/** One row of a typeahead. A tag's row leads with its kind's glyph. */
function Row({ item, on, onHover, onChoose }: { item: Item; on: boolean; onHover: () => void; onChoose: () => void }) {
  const Glyph = item.tag === undefined ? undefined : TAG_GLYPH[item.tag.kind];
  return (
    <div
      role="option"
      aria-selected={on}
      className={on ? "armada-mention__row armada-mention__row--active" : "armada-mention__row"}
      ref={(node) => {
        if (on) node?.scrollIntoView?.({ block: "nearest" });
      }}
      onMouseEnter={onHover}
      onMouseDown={(event) => {
        event.preventDefault();
        onChoose();
      }}
    >
      {Glyph === undefined ? null : <Glyph size={12} strokeWidth={2} aria-hidden className="armada-session-composer__glyph" />}
      <span className="armada-session-composer__name">{item.name}</span>
      {item.says === undefined ? null : <span className="armada-session-composer__says">{item.says}</span>}
    </div>
  );
}

/**
 * A select with its tooltip, and below the breakpoint a glyph in front of it
 * naming what it sets, so the row is a glyph and a value and not a label.
 */
function Pick({ Glyph, compact, label, children }: { Glyph: LucideIcon; compact: boolean; label: string; children: ReactNode }) {
  return (
    <Tooltip label={label}>
      <span className="armada-session-composer__pick" data-compact={compact || undefined}>
        {compact ? <Glyph size={12} strokeWidth={2} aria-hidden className="armada-session-composer__pick-glyph" /> : null}
        {children}
      </span>
    </Tooltip>
  );
}
