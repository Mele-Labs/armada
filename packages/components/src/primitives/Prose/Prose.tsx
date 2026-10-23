import { Children, cloneElement, createContext, isValidElement, memo, useContext, type ReactNode } from "react";
import Markdown, { defaultUrlTransform, type Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import { unified } from "unified";

import { Tooltip } from "../Tooltip/Tooltip";
import { commandOf, CopyAct, ProseRunner, RunnableBlock } from "./RunnableBlock";
import { isSlashCommand, ProseCommand, SlashCommand } from "./SlashCommand";
export type { CodePhase, CodeRun, CodeRunEvent, CodeRunner, TerminalSize } from "./RunnableBlock";
export { CopyAct, ProseRunner, RunnableBlock, RunOutput, useCodeRun } from "./RunnableBlock";
export { plainOf } from "./plain";
export type { DockedShell, ProseTerminalValue, TerminalDockRun } from "./TerminalRun";
export { DockedTerminal, ProseTerminal, useTerminalDock } from "./TerminalRun";
export { isSlashCommand, ProseCommand } from "./SlashCommand";
export type { SlashCommands } from "./SlashCommand";

/**
 * Text a model wrote, drawn as the markdown it is written in, with the GFM extensions,
 * through `react-markdown` and `remark-gfm`. A single newline is a line break, as
 * a forge draws a comment (`remark-breaks`). What each construct is drawn as is
 * the contract's — `docs/contracts/design-system.md`, *Prose — model-written markdown*.
 *
 * Raw HTML stays its characters: without `rehype-raw`, `react-markdown` turns
 * every HTML node back into text, and this text arrives from a model.
 */
export type ProseProps = {
  /** The text as it arrived. Empty draws nothing rather than an empty block. */
  text: string;
  /** Each top-level block carries `data-line`, the line of the text it starts on, for a view that scrolls with its source. */
  lines?: boolean;
};

/**
 * What opens a link, given an address already checked to be `http(s):`.
 * Without one a link draws as its text: no control where there is nowhere to
 * go. Bridge provides it once,
 * at the root of `App`.
 */
export const ProseLinks = createContext<((address: string) => void) | null>(null);

/**
 * Where a walk link opens: the walk window of the Session whose words these are. **A walk link is Armada's own
 * address for a walk**, `http://localhost` or `http://127.0.0.1` on any port with a `walk` query parameter.
 * Provided by a Session's thread; without one such a link is a web link like any other.
 */
export const ProseWalk = createContext<((address: string) => void) | null>(null);

function isWalkLink(address: string): boolean {
  try {
    const url = new URL(address);
    return url.protocol === "http:" && (url.hostname === "localhost" || url.hostname === "127.0.0.1") && url.searchParams.has("walk");
  } catch {
    return false;
  }
}

/**
 * Something in the words that Bridge itself can open: a pull request or an issue by its number, a
 * Session, a file in the Job's checkout by its path, or a web address the person taught Armada to draw words as (`link`,
 * whose `id` is the address). **A thing the host found, never one the text claims**: `scan` is the host's and reads its own state,
 * so a number that names nothing it holds stays plain text.
 */
export type ProseReference = {
  kind: "pull_request" | "issue" | "session" | "file" | "link";
  id: string;
  title: string;
};

/** What each kind is called, in the tooltip that leads a reference's title. */
export const REFERENCE_KIND: Record<ProseReference["kind"], string> = { pull_request: "Pull request", issue: "Issue", session: "Job", file: "File", link: "Link" };

/**
 * Where a reference is found and where it goes. Provided by the one surface that wants its words linked,
 * Helm's thread: every other `Prose` leaves its words as they are. `scan` returns the non-overlapping spans of
 * `text` that name something, in any order.
 */
export type ProseReferences = {
  scan: (text: string) => readonly { from: number; to: number; ref: ProseReference }[];
  open: (ref: ProseReference) => void;
};
export const ProseRefs = createContext<ProseReferences | null>(null);

/** The scheme a reference travels through the markdown in, `armada-ref:<kind>/<id>?t=<title>`. */
const REF = "armada-ref:";

/** A part of an address, with the brackets a markdown link would end on written out too. */
const escaped = (part: string): string => encodeURIComponent(part).replace(/[()]/g, (one) => `%${one.charCodeAt(0).toString(16)}`);

const addressOf = (ref: ProseReference): string => `${REF}${ref.kind}/${escaped(ref.id)}?t=${escaped(ref.title)}`;

/** A link a model wrote, `[words](address)`. */
const WRITTEN = /(!?\[(?:\\.|[^\[\]\\\n])*\]\([^()\s]*\))/;

/** The words with each reference they name written as a link, and code left alone. */
function linked(text: string, refs: ProseReferences): string {
  // A fenced block or an inline span is code, and a number in it is not a reference, unless the span is
  // exactly a Session or a file: `s-e165b4df` and `docs/plan.md` are how a model writes them, and draw as links, still as code.
  // A path in a sentence is only words.
  return text
    .split(/(```[\s\S]*?```|`[^`\n]*`)/)
    .map((part, at) => {
      if (at % 2 === 1) {
        const inner = part.startsWith("```") ? "" : part.slice(1, -1);
        const whole = inner === "" ? undefined : refs.scan(inner).find((one) => one.from === 0 && one.to === inner.length && (one.ref.kind === "session" || one.ref.kind === "file"));
        return whole === undefined ? part : `[${part}](${addressOf(whole.ref)})`;
      }
      // A link a model wrote is one link, and a reference in its words would be a link inside it, which markdown
      // draws as its brackets. One to a pull request or an issue the host holds opens where its number does.
      return part
        .split(WRITTEN)
        .map((piece, index) => {
          if (index % 2 === 0) return words(piece, refs);
          const address = piece.slice(piece.lastIndexOf("](") + 2, -1);
          const whole = refs.scan(address).find((one) => one.from === 0 && one.to === address.length && (one.ref.kind === "pull_request" || one.ref.kind === "issue"));
          return whole === undefined || piece.startsWith("!") ? piece : `${piece.slice(0, piece.length - address.length - 1)}${addressOf(whole.ref)})`;
        })
        .join("");
    })
    .join("");
}

/** Words outside code and outside a written link, each reference in them made a link. */
function words(part: string, refs: ProseReferences): string {
  const found = refs.scan(part).filter((one) => one.ref.kind !== "file").sort((a, b) => a.from - b.from);
  let out = "";
  let from = 0;
  for (const one of found) {
    if (one.from < from) continue;
    const named = part.slice(one.from, one.to).replace(/[\[\]]/g, "\\$&");
    out += `${part.slice(from, one.from)}[${named}](${addressOf(one.ref)})`;
    from = one.to;
  }
  return out + part.slice(from);
}

function referenceOf(href: string | undefined): ProseReference | null {
  if (href === undefined || !href.startsWith(REF)) return null;
  const [path = "", query = ""] = href.slice(REF.length).split("?t=");
  const [kind = "", id = ""] = path.split("/");
  if (!(kind in REFERENCE_KIND)) return null;
  return { kind: kind as ProseReference["kind"], id: decodeURIComponent(id), title: decodeURIComponent(query) };
}

/** A reference drawn as the link every other is, named on hover: its kind, then its title. */
export function ReferenceLink({ reference, children }: { reference: ProseReference; children: ReactNode }) {
  const refs = useContext(ProseRefs);
  if (refs === null) return <>{children}</>;
  return (
    <Tooltip label={`${REFERENCE_KIND[reference.kind]} · ${reference.title}`} asChild>
      <a
        className="armada-prose__link"
        href={`#${reference.kind}`}
        onClick={(event) => {
          event.preventDefault();
          refs.open(reference);
        }}
        onAuxClick={(event) => event.preventDefault()}
      >
        {children}
      </a>
    </Tooltip>
  );
}

const keepRef = (url: string): string => (url.startsWith(REF) ? url : defaultUrlTransform(url));

/**
 * **Memoised on its text**, which is its only prop: parsing markdown cost the owner's Cockpit 800 ms in a
 * profile (16 Oct 2026), most of it re-parsing messages whose words had not changed when a thread drew again.
 * What it reads from context still redraws it, and what its links read inside do not need it to.
 */
export const Prose = memo(function Prose({ text, lines = false }: ProseProps) {
  const refs = useContext(ProseRefs);
  if (text.trim() === "") return null;
  return (
    <div className="armada-prose">
      <Markdown remarkPlugins={lines ? LINED_PLUGINS : PLUGINS} components={lines ? LINED : DRAWN} urlTransform={keepRef}>
        {refs === null ? text : linked(text, refs)}
      </Markdown>
    </div>
  );
});

const PLUGINS = [remarkGfm, remarkBreaks];

/** Writes the line each top-level block starts on onto it, where `react-markdown` hands it to the component as `data-line`. */
function markLines() {
  return (tree: { children: { position?: { start: { line: number } }; data?: { hProperties?: Record<string, unknown> } }[] }) => {
    for (const block of tree.children) {
      if (block.position !== undefined) block.data = { ...block.data, hProperties: { ...block.data?.hProperties, "data-line": block.position.start.line } };
    }
  };
}
const LINED_PLUGINS = [...PLUGINS, markLines];

/** The address if it is one this app hands to a browser, else `null`. */
function web(href: string | undefined): string | null {
  if (!href) return null;
  try {
    const { protocol } = new URL(href);
    return protocol === "http:" || protocol === "https:" ? href : null;
  } catch {
    return null;
  }
}

/** Set inside a link, where a second one would be an anchor in an anchor. */
const InLink = createContext(false);

/** A web address in a span of code, drawn as the link it is. */
const ADDRESS = /https?:\/\/[^\s<>`]*[^\s<>`.,;:!?'")\]]/g;

function Addressed({ text, open }: { text: string; open: ((address: string) => void) | null }) {
  const walk = useContext(ProseWalk);
  const parts: ReactNode[] = [];
  let from = 0;
  for (const one of text.matchAll(ADDRESS)) {
    const address = web(one[0]);
    if (address === null || (open === null && !(walk !== null && isWalkLink(address)))) continue;
    parts.push(text.slice(from, one.index));
    parts.push(
      <WebLink key={one.index} address={address} open={open ?? walk!}>
        {address}
      </WebLink>,
    );
    from = one.index + one[0].length;
  }
  return <>{parts.length === 0 ? text : [...parts, text.slice(from)]}</>;
}

/** A web address drawn as a link; a walk link opens in the Session's walk window and says so. */
function WebLink({ address, open, children }: { address: string; open: (address: string) => void; children: ReactNode }) {
  const walk = useContext(ProseWalk);
  const inWindow = walk !== null && isWalkLink(address);
  const link = (
    <a
      className="armada-prose__link"
      href={address}
      {...(inWindow ? {} : { title: address })}
      onClick={(event) => {
        // No surface navigates: design-system.md, hard rule 2.
        event.preventDefault();
        (inWindow ? walk : open)(address);
      }}
      // A middle click routes around `onClick`; main denies the window anyway.
      onAuxClick={(event) => event.preventDefault()}
    >
      {children}
    </a>
  );
  return inWindow ? (
    <Tooltip label="Opens in the walk window" asChild>
      {link}
    </Tooltip>
  ) : (
    link
  );
}

function Link({ href, children }: { href: string | undefined; children: ReactNode }) {
  const open = useContext(ProseLinks);
  const walk = useContext(ProseWalk);
  const reference = referenceOf(href);
  if (reference !== null) return <InLink.Provider value><ReferenceLink reference={reference}>{children}</ReferenceLink></InLink.Provider>;
  const address = web(href);
  // A walk link needs the Session's window, not the browser, so it draws where only that is provided.
  if (address === null || (open === null && !(walk !== null && isWalkLink(address)))) return <>{children}</>;
  return (
    <InLink.Provider value>
      <WebLink address={address} open={open ?? walk!}>
        {children}
      </WebLink>
    </InLink.Provider>
  );
}

/** Set inside a fenced block, whose `code` is its text and never a command to send. */
const InBlock = createContext(false);

/** Inline code; one that is exactly a command the Session lists gains Run where a terminal Session provides it. */
function Code({ children }: { children?: ReactNode }) {
  const inBlock = useContext(InBlock);
  const inLink = useContext(InLink);
  const open = useContext(ProseLinks);
  const walk = useContext(ProseWalk);
  const commands = useContext(ProseCommand);
  if (inBlock || commands === null || typeof children !== "string" || !isSlashCommand(children, commands.names)) {
    // A web address is a link in code too; a `#12` in a block is a comment, so only an address is.
    return <code className="armada-prose__code">{inLink || typeof children !== "string" || (open === null && walk === null) ? children : <Addressed text={children} open={open} />}</code>;
  }
  return <SlashCommand text={children} send={commands.send} />;
}

/** A fenced block gains Copy; one in a shell language, or with none that reads as shell, gains Run too where a runner is provided. */
function Block({ children }: { children?: ReactNode }) {
  const runner = useContext(ProseRunner);
  const code = Children.toArray(children)[0];
  const language = isValidElement<{ className?: string }>(code) ? /language-(\S+)/.exec(code.props.className ?? "")?.[1] : undefined;
  const source = isValidElement<{ children?: ReactNode }>(code) && typeof code.props.children === "string" ? code.props.children : null;
  const command = runner === null || source === null ? null : commandOf(language, source);
  // The block leaves room for the acts drawn over its corner: one, or two where Run sits beside Copy.
  const pre = (
    <InBlock.Provider value>
      <pre className="armada-prose__block" data-acts={source === null ? undefined : command === null ? 1 : 2}>
        {children}
      </pre>
    </InBlock.Provider>
  );
  if (source === null) return pre;
  // The fence's closing newline is not the block's text: pasted into a terminal it would run the command.
  const copied = source.replace(/\n$/, "");
  if (command !== null) {
    return (
      <RunnableBlock command={command} runner={runner!} copy={copied}>
        {pre}
      </RunnableBlock>
    );
  }
  return (
    <div className="armada-prose__run">
      <div className="armada-prose__run-block">
        {pre}
        <span className="armada-prose__run-acts">
          <CopyAct text={copied} />
        </span>
      </div>
    </div>
  );
}

/** A heading of any level is a paragraph: a model's `#` inside a card is not the card's title. */
function said(level: number) {
  return function Said({ children }: { children?: ReactNode }) {
    return (
      <p className="armada-prose__said" data-level={level}>
        {children}
      </p>
    );
  };
}

/** Elements left out (`em`, `del`, `tr`, `thead`, `tbody`) carry no treatment of their own. */
const DRAWN: Components = {
  p: ({ children }) => <p className="armada-prose__paragraph">{children}</p>,
  h1: said(1),
  h2: said(2),
  h3: said(3),
  h4: said(4),
  h5: said(5),
  h6: said(6),
  strong: ({ children }) => <strong className="armada-prose__strong">{children}</strong>,
  code: Code,
  pre: Block,
  ul: ({ children }) => <ul className="armada-prose__list">{children}</ul>,
  ol: ({ children, start }) => (
    <ol className="armada-prose__list armada-prose__list--ordered" start={start}>
      {children}
    </ol>
  ),
  li: ({ children }) => <li className="armada-prose__item">{children}</li>,
  input: ({ checked }) => (
    <input
      className="armada-prose__task"
      type="checkbox"
      checked={checked ?? false}
      readOnly
      disabled
    />
  ),
  blockquote: ({ children }) => <blockquote className="armada-prose__quote">{children}</blockquote>,
  hr: () => <hr className="armada-prose__rule" />,
  table: ({ children }) => <table className="armada-prose__table">{children}</table>,
  // Column alignment is dropped: it arrives as an inline style, and nothing here takes one.
  th: ({ children }) => <th className="armada-prose__th">{children}</th>,
  td: ({ children }) => <td className="armada-prose__td">{children}</td>,
  a: ({ href, children }) => <Link href={href}>{children}</Link>,
  // A link, never a fetch: the CSP's `img-src 'self' blob:` would draw it broken.
  img: ({ src, alt }) => (
    <Link href={typeof src === "string" ? src : undefined}>{alt || src}</Link>
  ),
};

/** The parts of an mdast node `proseText` reads. */
type Node = { type: string; value?: string; alt?: string | null; children?: Node[] };

const PARSER = unified().use(remarkParse).use(remarkGfm);

/** Nodes whose end is a gap between words: blocks, cells, and a hard line break. */
const APART = new Set(["paragraph", "heading", "listItem", "tableCell", "code", "break"]);

/**
 * Model-written markdown as the plain words `Prose` would draw, for a place that
 * holds a string rather than a rendering, such as a title. Read by the same
 * parser, so an underscore inside a name stays and a link keeps its text.
 */
export function proseText(text: string): string {
  const words: string[] = [];
  const walk = (node: Node) => {
    words.push(node.type === "image" ? (node.alt ?? "") : (node.value ?? ""));
    node.children?.forEach(walk);
    if (APART.has(node.type)) words.push(" ");
  };
  walk(PARSER.parse(text) as Node);
  return words.join("").replace(/\s+/g, " ").trim();
}

/** `DRAWN`, each element it draws for a top-level block keeping the `data-line` the block was given. */
const LINED: Components = Object.fromEntries(
  Object.entries(DRAWN).map(([tag, Drawn]) => [
    tag,
    (props: Record<string, unknown>) => {
      const drawn = (Drawn as (props: Record<string, unknown>) => ReactNode)(props);
      const line = props["data-line"];
      return line === undefined || !isValidElement(drawn) ? drawn : cloneElement(drawn as React.ReactElement<Record<string, unknown>>, { "data-line": line });
    },
  ]),
);
