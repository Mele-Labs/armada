// What the request can tag, as the `@` and `#` menus draw it: the mention list's rows (`.armada-mention__row`), set in the sheet's flow so
// the frame, which clips what hangs out of it, never cuts it. `@` offers the live Sessions a brief can wait on and, once something follows
// it, the repository's files; `#` offers its open issues and pull requests by number. Used under the request while one is being typed and
// under the sentence when its Session is changed.

import { useEffect, useRef, useState, type ChangeEvent, type KeyboardEvent, type SyntheticEvent } from "react";
import { File, GitPullRequest, SquareTerminal, Ticket, type LucideIcon } from "lucide-react";
import type { ComposerNumbered, ComposerTag, NumberedHit } from "@armada/components";

export type Offered = { id: string; title: string };

/** One row of a menu: what it says, and the words it writes in place of what was typed. */
export type Row = { id: string; name: string; says?: string; standing?: string; glyph: ComposerTag["kind"] | "file"; insert: string; tag?: ComposerTag; /** The Session a Session's row stands for. */ session?: string };

/** What the menus read: the Sessions, and the files and numbered work of the repository the request is for. Both are absent until one is known. */
export type Tagging = {
  sessions: readonly Offered[];
  searchFiles?: (query: string) => Promise<readonly string[]>;
  numbered?: ComposerNumbered;
  /** An issue or a pull request was chosen: the words carry its reference, and it is sent with them. */
  onTag?: (tag: ComposerTag) => void;
};

/** The Sessions whose title holds what was typed after the `@`. */
export const matching = (sessions: readonly Offered[], query: string): Offered[] => sessions.filter((one) => one.title.toLowerCase().includes(query.trim().toLowerCase()));

const GLYPH: Record<Row["glyph"], LucideIcon> = { session: SquareTerminal, file: File, pull_request: GitPullRequest, issue: Ticket, branch: SquareTerminal };

/** How many file and numbered rows are offered at once, and how long a typed query rests before it is asked. */
const FILE_ROWS = 5;
const NUMBERED_ROWS = 6;
const REST_MS = 150;

const isNumber = (id: string): boolean => /^\d+$/.test(id);

/** How its tracker names it: `#2214` on the forge, the key itself (`ABC-12`) where the id is one. */
const referenceOf = (tag: ComposerTag): string => (isNumber(tag.id) ? `#${tag.id}` : tag.id);

export const sessionRow = (one: Offered): Row => ({ id: `session:${one.id}`, name: one.title, glyph: "session", insert: `@${one.title} `, session: one.id });

function fileRow(path: string): Row {
  const cut = path.lastIndexOf("/");
  return { id: `file:${path}`, name: path.slice(cut + 1), ...(cut < 0 ? {} : { says: path.slice(0, cut) }), glyph: "file", insert: `@${path} ` };
}

/** A `#` row: the reference in the name, the title after it. The words get the whole title, which leads with the reference. */
function numberedRow({ tag, standing }: NumberedHit): Row {
  const title = tag.title.startsWith(`${referenceOf(tag)} `) ? tag.title.slice(referenceOf(tag).length + 1) : tag.title;
  return { id: `${tag.kind}:${tag.id}`, name: referenceOf(tag), says: title, ...(standing === undefined ? {} : { standing }), glyph: tag.kind, insert: `${tag.title} `, tag };
}

/** Nothing typed offers everything; digits offer the numbered; letters offer the keys that begin so. */
function typed(tag: ComposerTag, query: string): boolean {
  if (query === "") return true;
  if (isNumber(query)) return isNumber(tag.id) && tag.id.startsWith(query);
  return !isNumber(tag.id) && referenceOf(tag).toLowerCase().startsWith(query.toLowerCase());
}

export function SessionMenu({ id, label, numbered = false, rows, active, onHover, onChoose }: { id: string; label: string; numbered?: boolean; rows: readonly Row[]; active: number; onHover: (at: number) => void; onChoose: (one: Row) => void }) {
  if (rows.length === 0) return null;
  return (
    <div className="armada-mention armada-dispatch-panel__menu" id={id} role="listbox" aria-label={label} data-numbered={numbered ? "" : undefined}>
      {rows.map((one, at) => {
        const Glyph = GLYPH[one.glyph];
        return (
          <div
            key={one.id}
            id={`${id}-${at}`}
            role="option"
            aria-selected={at === active}
            className={at === active ? "armada-mention__row armada-mention__row--active" : "armada-mention__row"}
            onMouseEnter={() => onHover(at)}
            // Not `onClick`: the press must not take the focus off the field the menu belongs to.
            onMouseDown={(event) => {
              event.preventDefault();
              onChoose(one);
            }}
          >
            <Glyph size={12} strokeWidth={2} aria-hidden />
            <span className={one.glyph === "session" ? undefined : "armada-session-composer__name"}>{one.name}</span>
            {one.says === undefined ? null : <span className="armada-session-composer__says">{one.says}</span>}
            {one.standing === undefined ? null : <span className="armada-session-composer__standing">{one.standing}</span>}
          </div>
        );
      })}
    </div>
  );
}

type Mention = { trigger: "@" | "#"; at: number; query: string };

/**
 * The `@` or `#` being typed in the request. `@` opens at one that starts a word and narrows as the Session's title is typed, whitespace
 * included while some title still starts with what was typed; what follows it also asks for files. `#` opens at one that starts a word
 * and narrows by the number or key typed. Arrows move, Enter or Tab picks, Esc closes it and nothing else.
 */
export function useAtMenu(words: string, onWords: (next: string) => void, tagging: Tagging, id: string) {
  const { sessions, searchFiles, numbered, onTag } = tagging;
  const [mention, setMention] = useState<Mention | null>(null);
  const [active, setActive] = useState(0);
  const field = useRef<HTMLTextAreaElement | null>(null);
  const [files, setFiles] = useState<readonly string[]>([]);
  const [found, setFound] = useState<{ number: number; hit: NumberedHit } | undefined>();
  // What the effects below ask is read where they run, so a new function each draw does not ask again.
  const reads = useRef({ searchFiles, numbered });
  reads.current = { searchFiles, numbered };

  function evaluate(el: HTMLTextAreaElement): void {
    field.current = el;
    const caret = el.selectionStart ?? el.value.length;
    const head = el.value.slice(0, caret);
    const at = head.lastIndexOf("@");
    const query = head.slice(at + 1);
    const starts = at === 0 || /\s/.test(head[at - 1] ?? "");
    const typing = !/\s/.test(query) || sessions.some((one) => one.title.toLowerCase().startsWith(query.toLowerCase()));
    if (at !== -1 && starts && !query.includes("\n") && typing) {
      setMention((was) => (was?.trigger === "@" && was.at === at && was.query === query ? was : { trigger: "@", at, query }));
      return setActive(0);
    }
    // Digits, or a key's letters and then its number: `#22`, `#ops`, `#OPS-7`.
    const hash = /(^|\s)#(\d*|[A-Za-z][A-Za-z0-9_]*(?:-\d*)?)$/.exec(head);
    if (hash === null || numbered === undefined) return setMention(null);
    const typedAfter = hash[2] ?? "";
    setMention((was) => (was?.trigger === "#" && was.query === typedAfter && was.at === head.length - typedAfter.length - 1 ? was : { trigger: "#", at: head.length - typedAfter.length - 1, query: typedAfter }));
    setActive(0);
  }

  const fileQuery = mention?.trigger === "@" && mention.query.trim() !== "" ? mention.query : undefined;
  useEffect(() => {
    const ask = reads.current.searchFiles;
    if (fileQuery === undefined || ask === undefined) return setFiles([]);
    let live = true;
    const timer = window.setTimeout(() => void ask(fileQuery).then((paths) => live && setFiles(paths)), REST_MS);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [fileQuery]);

  // A number that is not open is looked up once it has rested, and offered first when it resolves.
  const wanted = mention?.trigger === "#" && isNumber(mention.query) ? Number(mention.query) : undefined;
  useEffect(() => {
    const source = reads.current.numbered;
    if (wanted === undefined || source === undefined || source.open.some((one) => Number(one.tag.id) === wanted)) return setFound(undefined);
    let live = true;
    const timer = window.setTimeout(() => void source.find(wanted).then((hit) => live && setFound(hit === undefined ? undefined : { number: wanted, hit })), REST_MS);
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [wanted]);

  const offered: Row[] =
    mention === null
      ? []
      : mention.trigger === "@"
        ? [...matching(sessions, mention.query).map(sessionRow), ...(fileQuery === undefined ? [] : files.slice(0, FILE_ROWS).map(fileRow))]
        : [
            ...(found === undefined || found.number !== wanted ? [] : [found.hit]),
            ...(numbered?.open ?? [])
              .filter((one) => typed(one.tag, mention.query) && (found === undefined || found.number !== wanted || one.tag.id !== found.hit.tag.id))
              .sort((a, b) => (isNumber(a.tag.id) ? 1 : 0) - (isNumber(b.tag.id) ? 1 : 0) || (isNumber(a.tag.id) ? Number(b.tag.id) - Number(a.tag.id) : 0)),
          ]
            .filter((one) => !words.includes(one.tag.title))
            .slice(0, NUMBERED_ROWS)
            .map(numberedRow);
  const open = mention !== null && offered.length > 0;
  const label = mention?.trigger === "#" ? "Tag an issue or pull request" : files.length > 0 ? "Tag a Job or a file" : "Jobs";

  function choose(one: Row): void {
    const el = field.current;
    if (mention === null || el === null) return;
    const caret = mention.at + 1 + mention.query.length;
    onWords(`${words.slice(0, mention.at)}${one.insert}${words.slice(caret)}`);
    if (one.tag !== undefined) onTag?.(one.tag);
    setMention(null);
    const landed = mention.at + one.insert.length;
    // The value has not reached the field on this tick, and a range set now would be against the text it replaces.
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(landed, landed);
    });
  }

  /** True where the key was the menu's. */
  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>): boolean {
    field.current = event.currentTarget;
    if (!open) return false;
    if (event.key === "Escape") {
      event.preventDefault();
      setMention(null);
      return true;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      setActive((n) => (n + (event.key === "ArrowDown" ? 1 : -1) + offered.length) % offered.length);
      return true;
    }
    if ((event.key === "Enter" || event.key === "Tab") && !event.shiftKey && !event.altKey && !event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      const one = offered[active] ?? offered[0];
      if (one !== undefined) choose(one);
      return true;
    }
    return false;
  }

  return {
    open,
    offered,
    label,
    numbered: mention?.trigger === "#",
    active,
    setActive,
    choose,
    evaluate,
    close: () => setMention(null),
    listId: id,
    onKeyDown,
    onChange: (event: ChangeEvent<HTMLTextAreaElement>) => evaluate(event.target),
    onSelect: (event: SyntheticEvent<HTMLTextAreaElement>) => evaluate(event.currentTarget),
    onBlur: () => setMention(null),
    activeId: open ? `${id}-${active}` : undefined,
  };
}
