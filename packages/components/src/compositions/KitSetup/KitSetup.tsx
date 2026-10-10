import type { ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";

/**
 * The setup a person already works with, read from their agent harness's own
 * home and shown by kind — #1491.
 *
 * **Seeing is the whole of it.** Nothing here is a control, because nothing
 * here reaches a drone: a server a person connected outside Armada is drawn
 * and still handed to nobody, and allowing one stays a separate act on a kit
 * row below. A server is drawn as the program or host it is at and never as
 * what follows either, so what is on the screen could not start it.
 *
 * **The allowlist is the one row with a control.** Each command Armada keeps
 * there is drawn with where it came from, in words, and a Remove: that file is
 * Armada's own, so taking a line out is Armada's to do. It carries no count and
 * draws nothing under its name when it holds nothing.
 *
 * **A kind nothing reads yet says so.** Drawing it as empty would say a person
 * has none of something they have plenty of, which is the report this whole
 * surface exists to stop giving.
 */
export type KitSetupProps = {
  /**
   * What the read came to. `undefined` is not yet read and draws "Reading." —
   * an empty setup drawn before the read lands is the wrong answer given by
   * accident, `KitServers`' reason.
   */
  setup?: KitSetupRead;
  /** Take a command out of the allowlist, by the line as it was read. Absent draws no Remove. */
  onRemoveAllowed?: (run: string) => void;
};

export type KitSetupRead = {
  /** The harness, in its own name. **Drawn, never matched on.** */
  harness: string;
  /** Where it was read, as a person would type it. */
  home: string;
  /** Whether that home is there at all. */
  present: boolean;
  /** Every kind, in the order fleet answered them. */
  kinds: KitSetupKind[];
};

/** Armada's word for one kind of thing, never a harness's. */
export type KitSetupKindWord =
  | "skills"
  | "plugins"
  | "agent_file"
  | "sub_agents"
  | "commands"
  | "mcp_servers"
  | "allowlist"
  | "models";

export type KitSetupKind = {
  kind: KitSetupKindWord;
  read:
    | { what: "read"; items: KitSetupItem[]; unreadable: KitSetupUnreadable[] }
    | { what: "not_read"; why: string };
};

export type KitSetupItem = {
  name: string;
  /** Its own words for itself — or, for a server, the program or host it is at. */
  says?: string;
  /** Where it came from. Drawn small, and never a control. */
  source: string;
};

export type KitSetupUnreadable = {
  source: string;
  why: string;
};

const KIND_LABEL: Record<KitSetupKindWord, string> = {
  skills: "Skills",
  plugins: "Plugins",
  agent_file: "Agent file",
  sub_agents: "Sub agents",
  commands: "Commands",
  mcp_servers: "MCP servers",
  allowlist: "Allowlist",
  models: "Models",
};

/**
 * What a kind is, where a person would not already know. The servers line is
 * the one that matters: it is the difference between seeing a server and a
 * drone getting one.
 */
const KIND_NOTE: Partial<Record<KitSetupKindWord, string>> = {
  mcp_servers: "Connected outside Armada. A drone here is handed none of them — allow one in Kit below and it is a Kit server from then on.",
  agent_file: "The global file saying how you want an agent to behave.",
};

/** A kind with nothing in it, in that kind's own words. */
const KIND_NONE: Record<KitSetupKindWord, string> = {
  skills: "No skills",
  plugins: "No plugins",
  agent_file: "No agent file",
  sub_agents: "No sub agents",
  commands: "No commands",
  mcp_servers: "No servers connected",
  allowlist: "Nothing always-allowed",
  models: "No models named",
};

export function KitSetup({ setup, onRemoveAllowed }: KitSetupProps) {
  // Before the read answers there is nothing yet to say.
  if (setup === undefined) return null;
  return (
    <section className="armada-kit-setup" aria-label="What you already have">
      <header className="armada-kit-setup__head">
        <h3 className="armada-kit-setup__title">What you already have</h3>
        <p className="armada-kit-setup__where">
          {setup.harness}
          <span className="armada-kit-setup__home">{setup.home}</span>
        </p>
      </header>

      {/* The glass the kinds stand on, each a tile. */}
      <div className="armada-kit-setup__glass">
        {setup.kinds.map((kind) => (
          <Kind key={kind.kind} home={setup.home} onRemoveAllowed={onRemoveAllowed} {...kind} />
        ))}
      </div>
    </section>
  );
}

function Kind({
  kind,
  read,
  home,
  onRemoveAllowed,
}: KitSetupKind & { home: string; onRemoveAllowed?: ((run: string) => void) | undefined }) {
  // The allowlist draws no count: a count beside the list it counts is design-system hard rule 7.
  const counted = read.what === "read" && kind !== "allowlist" ? read.items.length : undefined;
  return (
    // A tile on the glass: focusable, so a reading with no control in it is still reached by the keys.
    <section className="armada-kit-setup__kind" aria-label={KIND_LABEL[kind]} tabIndex={0} data-kit-tile="">
      <h4 className="armada-kit-setup__kind-name">
        {KIND_LABEL[kind]}
        {counted === undefined ? null : (
          <span className="armada-kit-setup__count">{counted}</span>
        )}
      </h4>
      <Note>{KIND_NOTE[kind]}</Note>
      {read.what === "not_read" ? (
        /* Named as not read, never drawn as empty. */
        <p className="armada-kit-setup__not-read">Not read yet — {read.why}</p>
      ) : kind === "allowlist" ? (
        <Allowed items={read.items} onRemove={onRemoveAllowed} />
      ) : (
        <Items kind={kind} home={home} items={read.items} unreadable={read.unreadable} />
      )}
    </section>
  );
}

/**
 * Where an allowlist command came from, in words. **Fleet's own spelling is
 * `retro item <id>`, `always allow` or `written by hand`**, and the id is what
 * the retro page is for, so only the kind is drawn. A spelling this build has
 * no words for is drawn as it came.
 */
function sourceWords(source: string): string {
  if (source.startsWith("retro item")) return "Retro item";
  if (source === "always allow") return "Always allow";
  if (source === "written by hand") return "Written by hand";
  return source;
}

/**
 * The commands Kit allows for every Job on this machine. **Nothing under the
 * name where there are none**: an empty list says so by being empty.
 */
function Allowed({
  items,
  onRemove,
}: {
  items: KitSetupItem[];
  onRemove?: ((run: string) => void) | undefined;
}) {
  if (items.length === 0) return null;
  return (
    <ul className="armada-kit-setup__items">
      {items.map((item) => (
        <li className="armada-kit-setup__item" key={item.name}>
          <span className="armada-kit-setup__item-name mono">{item.name}</span>
          <span className="armada-kit-setup__item-from">{sourceWords(item.source)}</span>
          {onRemove === undefined ? null : (
            <Button
              variant="ghost"
              size="sm"
              aria-label={`Remove ${item.name}`}
              onClick={() => onRemove(item.name)}
            >
              Remove
            </Button>
          )}
        </li>
      ))}
    </ul>
  );
}

function Note({ children }: { children?: ReactNode }) {
  return children === undefined ? null : <p className="armada-kit-setup__note">{children}</p>;
}

/**
 * A path under the home, with the home taken off: it is written once at the
 * top, and repeating it on every row pushes the thing itself off the line.
 */
function under(source: string, home: string): string {
  return source.startsWith(`${home}/`) ? source.slice(home.length + 1) : source;
}

function Items({
  kind,
  home,
  items,
  unreadable,
}: {
  kind: KitSetupKindWord;
  home: string;
  items: KitSetupItem[];
  unreadable: KitSetupUnreadable[];
}) {
  return (
    <>
      {items.length === 0 && unreadable.length === 0 ? (
        <p className="armada-kit-setup__none">{KIND_NONE[kind]}</p>
      ) : (
        <ul className="armada-kit-setup__items">
          {items.map((item) => (
            <li className="armada-kit-setup__item" key={`${item.source}/${item.name}`}>
              <span className="armada-kit-setup__item-name">{item.name}</span>
              {item.says === undefined ? null : (
                /* One line, and the whole of it on hover: twenty-one rows each
                   three lines deep is a list nobody reads to the end of. */
                <span className="armada-kit-setup__item-says" title={item.says}>
                  {item.says}
                </span>
              )}
              <span className="armada-kit-setup__item-source" title={item.source}>
                {under(item.source, home)}
              </span>
            </li>
          ))}
        </ul>
      )}
      {unreadable.length === 0 ? null : (
        /* A file Armada could not describe is still one the person has, so it
           is named with the reason and nothing is offered to fix it here. */
        <ul className="armada-kit-setup__unreadable">
          {unreadable.map((one) => (
            <li key={one.source}>
              <span className="armada-kit-setup__item-name" title={one.source}>
                {under(one.source, home)}
              </span>
              <span className="armada-kit-setup__item-says">would not read: {one.why}</span>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}
