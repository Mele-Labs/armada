import { createContext, useContext, type ReactNode } from "react";
import Markdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";

/**
 * Text a model wrote, drawn as the markdown it is written in: GitHub's flavour,
 * through `react-markdown` and `remark-gfm`. What each construct is drawn as,
 * and why the renderer declares no font size, is the contract's —
 * `docs/contracts/design-system.md`, *Prose — model-written markdown*.
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

export function Prose({ text }: ProseProps) {
  if (text.trim() === "") return null;
  return (
    <div className="armada-prose">
      <Markdown remarkPlugins={[remarkGfm]} components={DRAWN}>
        {text}
      </Markdown>
    </div>
  );
}

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

/** Every heading level: the line at heading weight, with no scale and no outline entry. */
function said({ children }: { children?: ReactNode }) {
  return <p className="armada-prose__said">{children}</p>;
}

/** Elements left out (`em`, `del`, `tr`, `thead`, `tbody`) carry no treatment of their own. */
const DRAWN: Components = {
  p: ({ children }) => <p className="armada-prose__paragraph">{children}</p>,
  h1: said,
  h2: said,
  h3: said,
  h4: said,
  h5: said,
  h6: said,
  strong: ({ children }) => <strong className="armada-prose__strong">{children}</strong>,
  code: ({ children }) => <code className="armada-prose__code">{children}</code>,
  pre: ({ children }) => <pre className="armada-prose__block">{children}</pre>,
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
