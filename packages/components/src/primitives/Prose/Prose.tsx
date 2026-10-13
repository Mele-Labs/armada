import { Children, createContext, isValidElement, useContext, type ReactNode } from "react";
import Markdown, { defaultUrlTransform, type Components } from "react-markdown";
import remarkBreaks from "remark-breaks";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import { unified } from "unified";

import { Tooltip } from "../Tooltip/Tooltip";
import { commandOf, ProseRunner, RunnableBlock } from "./RunnableBlock";
import { isSlashCommand, ProseCommand, SlashCommand } from "./SlashCommand";
export type { CodeRunEvent, CodeRunner } from "./RunnableBlock";
export { ProseRunner } from "./RunnableBlock";
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
};

/**
 * What opens a link, given an address already checked to be `http(s):`.
 * Without one a link draws as its text, `JobDetailHeaderActions`' rule for
 * `href`: no control where there is nowhere to go. Bridge provides it once,
 * at the root of `App`.
 */
export const ProseLinks = createContext<((address: string) => void) | null>(null);

/**
 * Something in the words that Bridge itself can open: a pull request or an issue by its number, a Job, a
 * Session. **A thing the host found, never one the text claims**: `scan` is the host's and reads its own state,
 * so a number that names nothing it holds stays plain text.
 */
export type ProseReference = {
  kind: "pull_request" | "issue" | "job" | "session";
  id: string;
  title: string;
};

/** What each kind is called, in the tooltip that leads a reference's title. */
export const REFERENCE_KIND: Record<ProseReference["kind"], string> = { pull_request: "Pull request", issue: "Issue", job: "Job", session: "Session" };

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

/** The words with each reference they name written as a link, and code left alone. */
function linked(text: string, refs: ProseReferences): string {
  // A fenced block or an inline span is code, and a number in it is not a reference.
  return text
    .split(/(```[\s\S]*?```|`[^`\n]*`)/)
    .map((part, at) => {
      if (at % 2 === 1) return part;
      const found = [...refs.scan(part)].sort((a, b) => a.from - b.from);
      let out = "";
      let from = 0;
      for (const one of found) {
        if (one.from < from) continue;
        const words = part.slice(one.from, one.to).replace(/[\[\]]/g, "\\$&");
        out += `${part.slice(from, one.from)}[${words}](${REF}${one.ref.kind}/${escaped(one.ref.id)}?t=${escaped(one.ref.title)})`;
        from = one.to;
      }
      return out + part.slice(from);
    })
    .join("");
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

export function Prose({ text }: ProseProps) {
  const refs = useContext(ProseRefs);
  if (text.trim() === "") return null;
  return (
    <div className="armada-prose">
      <Markdown remarkPlugins={PLUGINS} components={DRAWN} urlTransform={(url) => (url.startsWith(REF) ? url : defaultUrlTransform(url))}>
        {refs === null ? text : linked(text, refs)}
      </Markdown>
    </div>
  );
}

const PLUGINS = [remarkGfm, remarkBreaks];

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

function Link({ href, children }: { href: string | undefined; children: ReactNode }) {
  const open = useContext(ProseLinks);
  const reference = referenceOf(href);
  if (reference !== null) return <ReferenceLink reference={reference}>{children}</ReferenceLink>;
  const address = web(href);
  if (address === null || open === null) return <>{children}</>;
  return (
    <a
      className="armada-prose__link"
      href={address}
      title={address}
      onClick={(event) => {
        // No surface navigates: design-system.md, hard rule 2.
        event.preventDefault();
        open(address);
      }}
      // A middle click routes around `onClick`; main denies the window anyway.
      onAuxClick={(event) => event.preventDefault()}
    >
      {children}
    </a>
  );
}

/** Set inside a fenced block, whose `code` is its text and never a command to send. */
const InBlock = createContext(false);

/** Inline code; one that is exactly a command the Session lists gains Run where a terminal Session provides it. */
function Code({ children }: { children?: ReactNode }) {
  const inBlock = useContext(InBlock);
  const commands = useContext(ProseCommand);
  if (inBlock || commands === null || typeof children !== "string" || !isSlashCommand(children, commands.names)) {
    return <code className="armada-prose__code">{children}</code>;
  }
  return <SlashCommand text={children} send={commands.send} />;
}

/** A fenced block; one in a shell language, or with none that reads as shell, gains Run where a runner is provided. */
function Block({ children }: { children?: ReactNode }) {
  const runner = useContext(ProseRunner);
  const pre = (
    <InBlock.Provider value>
      <pre className="armada-prose__block">{children}</pre>
    </InBlock.Provider>
  );
  if (runner === null) return pre;
  const code = Children.toArray(children)[0];
  if (!isValidElement<{ className?: string; children?: ReactNode }>(code)) return pre;
  const language = /language-(\S+)/.exec(code.props.className ?? "")?.[1];
  const source = typeof code.props.children === "string" ? code.props.children : null;
  const command = source === null ? null : commandOf(language, source);
  if (command === null) return pre;
  return (
    <RunnableBlock command={command} runner={runner}>
      {pre}
    </RunnableBlock>
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
