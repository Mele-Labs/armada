import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { ClipboardEvent, DragEvent, FormEvent, KeyboardEvent, ReactNode } from "react";
import { Cpu, Paperclip, PencilRuler, Send, Shield, Zap } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { useKept, writeKept } from "../../keep";
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
 * **`/` opens the skills and commands, `@` opens the other Sessions** and **`#` the repository's
 * issues and pull requests by number**; choosing one writes it into the message as a chip, at the
 * caret, so the agent knows what is meant. **A
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

/**
 * An issue or a pull request `#` offers, as a tag. `standing` is set on one that is not open (`Merged`,
 * `Closed`), which only a lookup by number turns up.
 */
export type NumberedHit = { tag: ComposerTag; standing?: string };

/**
 * What `#` reads. **`open` is the repository's open issues and pull requests**, in any order. `find` looks
 * one number up when it is not among them, and resolves to nothing where the repository has none.
 */
export type ComposerNumbered = {
  open: readonly NumberedHit[];
  find: (number: number) => Promise<NumberedHit | undefined>;
};

/** How many rows `#` offers at once, and how long a typed number rests before it is looked up. */
const NUMBERED_ROWS = 6;
const LOOKUP_MS = 150;

/** What each kind is called, on the glyph that leads a `#` row. */
const KIND_SAID: Record<ComposerTag["kind"], string> = { session: "Session", job: "Job", pull_request: "Pull request", branch: "Branch", issue: "Issue" };

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
  /** A turn is running. Send stays on: a message sent now is the agent's next input. */
  working: boolean;
  /** Absent with `onMode` draws no permission mode: Helm keeps its own rule. */
  mode?: ComposerMode;
  onMode?: (mode: ComposerMode) => void;
  model: string | null;
  effort: string | null;
  models: readonly string[];
  efforts: readonly string[];
  onTune: (tuning: { model: string | null; effort: string | null }) => void;
  /** What `/` offers. */
  commands: readonly { name: string; says: string }[];
  /** What `@` offers, grouped by kind: other Sessions, Jobs, pull requests and branches. */
  taggable: readonly ComposerTag[];
  /** What `#` offers for the repository this box speaks for. Absent, `#` is only a character. */
  numbered?: ComposerNumbered;
  /** The box's accessible name. */
  label?: string;
  /** Nothing can be sent: the box takes no focus and Send is off. */
  disabled?: boolean;
  /** Send's weight. Helm's is quieter than the one Primary a view is allowed. */
  send?: "primary" | "secondary";
  /** Send is its glyph, named on hover, so the row holds in a narrow dock. */
  slim?: boolean;
  /**
   * What the box takes half of before it scrolls: the panel it sits in, or the nearest scrolling
   * container above it, which is a dock's, whose own wrapper is no taller than the box.
   */
  grows?: "panel" | "scroller";
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
  /** Resolving to `false` says the message did not go, and the box is given it back. */
  onSend: (sent: SentFromComposer) => void | Promise<boolean>;
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
  /** The `draftKey` whose box takes focus, with the caret after what it holds, when it shows: a fork opens on its quote. */
  focusFor?: string;
};

/** The message: its text holds a CHIP where each tag stands, and `tags` lists them in order. */
type Draft = { text: string; tags: ComposerTag[]; files: ComposerFile[] };
const BLANK: Draft = { text: "", tags: [], files: [] };

/**
 * Puts `text` in the box kept for a Session, as though it had been typed there and not sent.
 * **A line break that ends the box is only where the last line stops**, and typing there takes it
 * away, so one more is written: a quote and a blank line stay two lines apart under the ask.
 */
export function seedDraft(draftKey: string, text: string): void {
  const seeded = text.endsWith("\n") ? `${text}\n` : text;
  writeKept<Draft>(`session:${draftKey}`, { text: seeded, tags: [], files: [] }, { text: seeded, tags: [] });
}

/** The commands a `/` query keeps: names that start with it first, then names that hold it. */
function matching<T extends { name: string }>(commands: readonly T[], query: string): T[] {
  const wanted = query.toLowerCase();
  const starts = commands.filter((one) => one.name.toLowerCase().startsWith(wanted));
  const holds = commands.filter((one) => !one.name.toLowerCase().startsWith(wanted) && one.name.toLowerCase().includes(wanted));
  return [...starts, ...holds];
}

type Item = { id: string; name: string; says?: string; standing?: string; tag?: ComposerTag };

/** A `#` row: the number in the name, the title after it. */
const numberedRow = (hit: NumberedHit): Item => ({
  id: `${hit.tag.kind}${hit.tag.id}`,
  name: `#${hit.tag.id}`,
  says: hit.tag.title.replace(/^#\d+\s*/, ""),
  ...(hit.standing === undefined ? {} : { standing: hit.standing }),
  tag: hit.tag,
});

/** What the caret is in: a `/` at the start of the message, or an `@` or a `#` at the start of a word. */
function tokenAt(text: string, caret: number): { trigger: "/" | "@" | "#"; query: string; from: number } | undefined {
  const head = text.slice(0, caret);
  const slash = /^\/(\S*)$/.exec(head);
  if (slash !== null) return { trigger: "/", query: slash[1] ?? "", from: 0 };
  const at = /(^|\s)@(\S*)$/.exec(head);
  if (at !== null) return { trigger: "@", query: at[2] ?? "", from: head.length - (at[2]?.length ?? 0) - 1 };
  const hash = /(^|\s)#(\d*)$/.exec(head);
  if (hash !== null) return { trigger: "#", query: hash[2] ?? "", from: head.length - (hash[2]?.length ?? 0) - 1 };
  return undefined;
}

/**
 * What `#` offers for a number typed so far: the open ones that start with it, newest number first,
 * with a number looked up on its own coming ahead of them. A tag already in the message is not offered twice.
 */
function numberedItems(numbered: ComposerNumbered | undefined, query: string, looked: NumberedHit | undefined, held: readonly ComposerTag[]): Item[] {
  if (numbered === undefined) return [];
  const has = (tag: ComposerTag) => held.some((had) => had.kind === tag.kind && had.id === tag.id);
  const hits = [
    ...(looked === undefined ? [] : [looked]),
    ...numbered.open
      .filter((one) => one.tag.id.startsWith(query) && (looked === undefined || one.tag.id !== looked.tag.id))
      .sort((a, b) => Number(b.tag.id) - Number(a.tag.id)),
  ];
  return hits.filter((one) => !has(one.tag)).slice(0, NUMBERED_ROWS).map(numberedRow);
}

export function SessionComposer({
  mode = "auto",
  onMode,
  model,
  effort,
  models,
  efforts,
  onTune,
  commands,
  taggable,
  numbered,
  label = "Message",
  disabled = false,
  send: sendWeight = "primary",
  slim = false,
  grows = "panel",
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
  focusFor,
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
  const wanted = token?.trigger === "#" && token.query !== "" ? Number(token.query) : undefined;
  // A number that is not open is looked up once it has rested, and offered first when it resolves.
  const [found, setFound] = useState<{ number: number; hit: NumberedHit } | undefined>();
  const source = useRef(numbered);
  source.current = numbered;
  useEffect(() => {
    const reads = source.current;
    if (wanted === undefined || reads === undefined || reads.open.some((one) => Number(one.tag.id) === wanted)) {
      setFound(undefined);
      return;
    }
    let live = true;
    const timer = window.setTimeout(() => {
      void reads.find(wanted).then((hit) => {
        if (live) setFound(hit === undefined ? undefined : { number: wanted, hit });
      });
    }, LOOKUP_MS);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [wanted]);
  const items: Item[] =
    token === undefined
      ? []
      : token.trigger === "/"
        ? matching(commands, token.query).map((one) => ({ id: one.name, name: `/${one.name}`, ...(one.says === "" ? {} : { says: one.says }) }))
        : token.trigger === "#"
          ? numberedItems(numbered, token.query, wanted === undefined || found?.number !== wanted ? undefined : found.hit, draft.tags)
          : TAG_KINDS.flatMap(({ kind }) =>
            taggable
              .filter((one) => one.kind === kind && !draft.tags.some((had) => had.kind === kind && had.id === one.id) && one.title.toLowerCase().includes(token.query.toLowerCase()))
              .map((one) => ({ id: `${kind}${one.id}`, name: one.title, tag: one })),
          );
  const open = items.length > 0;
  // The list hangs above the box, where a box at the foot of its window has the room. In Helm's dock a box
  // with no thread over it has none, and the dock clips what leaves it: then it hangs below.
  const form = useRef<HTMLFormElement>(null);
  const [below, setBelow] = useState(false);
  useLayoutEffect(() => {
    const box = form.current;
    if (!open || box === null) return;
    let clip: HTMLElement | null = box.parentElement;
    while (clip !== null && getComputedStyle(clip).overflowY === "visible") clip = clip.parentElement;
    const bound = clip?.getBoundingClientRect() ?? { top: 0, bottom: window.innerHeight };
    const at = box.getBoundingClientRect();
    const needs = Math.min(items.length, 6) * 28 + 16;
    setBelow(at.top - bound.top < needs && bound.bottom - at.bottom > at.top - bound.top);
  }, [open, items.length]);
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

  // After the draw above, so the caret lands behind what the box was just given.
  useEffect(() => {
    const box = editor.current;
    if (box === null || focusFor === undefined || focusFor !== draftKey) return;
    box.focus();
    caretToEnd(box);
  }, [focusFor, draftKey]);

  // The box takes the most of half the panel it sits in, then scrolls.
  useLayoutEffect(() => {
    const box = editor.current;
    if (box === null) return;
    let bound = box.closest("form")?.parentElement ?? null;
    if (grows === "scroller") while (bound !== null && !/auto|scroll|hidden/.test(getComputedStyle(bound).overflowY)) bound = bound.parentElement;
    box.style.maxHeight = bound ? `${bound.clientHeight / 2}px` : "";
  }, [draft.text, grows]);

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
    if (disabled) return;
    add([...event.dataTransfer.files]);
  };

  const held = files.length + drawn.length > 0;

  const send = () => {
    if (disabled || (text.trim() === "" && !held)) return;
    const sent = draft;
    const done = onSend({ text: sentText(text, draft.tags).trim(), files, tags: draft.tags });
    forget();
    setHead("");
    // A refused message comes back, unless something new has been typed since.
    void done?.then((went) => {
      if (!went) setDraft((now) => (now.text === "" && now.tags.length === 0 && now.files.length === 0 ? sent : now));
    });
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    send();
  };

  const chips = (
    <div className="armada-session-composer__chips" role="group" aria-label="Attached">
      {files.map((one) => (
        <AttachmentChip key={one.id} filename={one.name} {...(one.src === undefined ? {} : { preview: one.src })} onRemove={() => setFiles((was) => was.filter((f) => f.id !== one.id))} />
      ))}
      {drawn.map((one) => (
        <AttachmentChip key={one.id} filename={one.title} from="Sketch" onRemove={() => onRemoveDrawn(one.id)} />
      ))}
    </div>
  );
  const sendButton = (
    <Button type="submit" variant={sendWeight} size="sm" {...(slim ? { "aria-label": "Send" } : {})} disabled={disabled || (text.trim() === "" && !held)}>
      <Send size={12} strokeWidth={2} aria-hidden />
      {slim ? null : "Send"}
    </Button>
  );
  const said = MODES.find((one) => one.id === mode)!.says;
  return (
    <form ref={form} className="armada-session-composer" onSubmit={submit} onDragOver={(event) => event.preventDefault()} onDrop={onDrop}>
      {!open ? null : (
        <div className="armada-mention armada-session-composer__offer" data-below={below ? "" : undefined} id={listId} role="listbox" aria-label={token?.trigger === "/" ? "Skills and commands" : token?.trigger === "#" ? "Tag an issue or pull request" : "Tag a Session, Job, pull request or branch"} data-numbered={token?.trigger === "#" ? "" : undefined}>
          {token?.trigger === "/" || token?.trigger === "#"
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
          aria-label={label}
          aria-multiline
          aria-disabled={disabled || undefined}
          aria-controls={open ? listId : undefined}
          contentEditable={!disabled}
          suppressContentEditableWarning
          className="armada-textarea armada-session-composer__editor"
          onInput={read}
          onKeyDown={onKeyDown}
          onPaste={onPaste}
        />
      </div>
      {compact && held ? <div className="armada-session-composer__held">{chips}</div> : null}
      <div className="armada-session-composer__bar">
        {modeHidden || onMode === undefined ? null : (
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
        {compact && held ? null : chips}
        {slim ? <Tooltip label="Send">{sendButton}</Tooltip> : sendButton}
      </div>
    </form>
  );
}

/** One row of a typeahead. A tag's row leads with its kind's glyph. */
function Row({ item, on, onHover, onChoose }: { item: Item; on: boolean; onHover: () => void; onChoose: () => void }) {
  const Glyph = item.tag === undefined ? undefined : TAG_GLYPH[item.tag.kind];
  // A `#` row's glyph is the only word for its kind, so it is named on hover.
  const kind = item.tag !== undefined && item.name.startsWith("#") ? KIND_SAID[item.tag.kind] : undefined;
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
      {Glyph === undefined ? null : kind === undefined ? (
        <Glyph size={12} strokeWidth={2} aria-hidden className="armada-session-composer__glyph" />
      ) : (
        <Tooltip label={kind}>
          <span role="img" aria-label={kind} className="armada-session-composer__glyph">
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      )}
      <span className="armada-session-composer__name">{item.name}</span>
      {item.says === undefined ? null : <span className="armada-session-composer__says">{item.says}</span>}
      {item.standing === undefined ? null : <span className="armada-session-composer__standing">{item.standing}</span>}
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
