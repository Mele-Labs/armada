import { useId, useRef, useState } from "react";
import type { ClipboardEvent, DragEvent, FormEvent, KeyboardEvent, ReactNode } from "react";
import { Box, Cpu, GitBranch, GitPullRequest, Paperclip, PencilRuler, Send, Shield, SquareTerminal, Zap } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { AttachmentChip } from "../../primitives/AttachmentChip/AttachmentChip";
import { Button } from "../../primitives/Button/Button";
import { Select } from "../../primitives/Select/Select";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * A Session's message box, with what a terminal session's has, in **one box
 * and one slim row under the typing area**. The row holds the permission mode,
 * Dispatch's own model and effort pair, an act to attach a file and one to draw
 * a sketch, then what is waiting to be sent in a single row that scrolls
 * sideways, so nothing waiting ever takes room from the typing area.
 *
 * **`/` opens the skills and commands, and `@` opens the other Sessions**,
 * choosing one tags it with a chip so the agent knows to talk to it. **A
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

/** Something `@` tags: another Session, a Job, a pull request or a branch. */
export type ComposerTag = { kind: "session" | "job" | "pull_request" | "branch"; id: string; title: string };

export type SentFromComposer = {
  text: string;
  files: readonly ComposerFile[];
  tags: readonly ComposerTag[];
};

const TAG_GLYPH: Record<ComposerTag["kind"], LucideIcon> = {
  session: SquareTerminal,
  job: Box,
  pull_request: GitPullRequest,
  branch: GitBranch,
};

/** The kinds in the order `@` groups them, each named for the group's label. */
const TAG_KINDS: { kind: ComposerTag["kind"]; label: string }[] = [
  { kind: "job", label: "Jobs" },
  { kind: "pull_request", label: "Pull requests" },
  { kind: "branch", label: "Branches" },
  { kind: "session", label: "Sessions" },
];

const TAG_NAME: Record<ComposerTag["kind"], string> = { session: "Session", job: "Job", pull_request: "Pull request", branch: "Branch" };

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
  /** What has been tagged and waits to be sent. */
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
};

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
}: SessionComposerProps) {
  const [text, setText] = useState("");
  const [caret, setCaret] = useState(0);
  const [files, setFiles] = useState<ComposerFile[]>([]);
  const [active, setActive] = useState(0);
  const picker = useRef<HTMLInputElement>(null);
  const counter = useRef(0);
  const listId = useId();

  const token = tokenAt(text, caret);
  const items: Item[] =
    token === undefined
      ? []
      : token.trigger === "/"
        ? commands.filter((one) => one.name.toLowerCase().includes(token.query.toLowerCase())).map((one) => ({ id: one.name, name: `/${one.name}`, says: one.says }))
        : TAG_KINDS.flatMap(({ kind }) =>
            taggable
              .filter((one) => one.kind === kind && !tags.some((had) => had.kind === kind && had.id === one.id) && one.title.toLowerCase().includes(token.query.toLowerCase()))
              .map((one) => ({ id: `${kind}${one.id}`, name: one.title, tag: one })),
          );
  const open = items.length > 0;
  const current = Math.min(active, Math.max(items.length - 1, 0));

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
    if (token === undefined) return;
    const after = text.slice(caret);
    if (token.trigger === "/") {
      setText(`/${item.id} ${after}`);
      setCaret(item.id.length + 2);
    } else {
      setText(`${text.slice(0, token.from)}${after}`);
      setCaret(token.from);
      if (item.tag !== undefined) onTags([...tags, item.tag]);
    }
    // The press was on a row that took no focus (mousedown is prevented), so the box still has it.
    setActive(0);
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

  const held = files.length + drawn.length + tags.length > 0;

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (working || (text.trim() === "" && !held)) return;
    onSend({ text: text.trim(), files, tags });
    setText("");
    setCaret(0);
    setFiles([]);
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
      <div className="armada-session-composer__bar">
        <Pick Glyph={Shield} compact={compact} label={said}>
          <Select aria-label="Permission mode" value={mode} onChange={(event) => onMode(event.target.value as ComposerMode)}>
            {MODES.map((one) => (
              <option key={one.id} value={one.id}>
                {one.label}
              </option>
            ))}
          </Select>
        </Pick>
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
          {tags.map((one) => (
            <AttachmentChip key={`${one.kind}${one.id}`} filename={one.title} from={TAG_NAME[one.kind]} onRemove={() => onTags(tags.filter((had) => had !== one))} />
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
