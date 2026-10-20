import { useLayoutEffect, useRef, useState } from "react";
import type { ReactNode } from "react";

import { Check } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { SharedHomeBar, SyncAct, SyncMarks, SyncSheet, useSharedHome } from "./SharedHome";
import type { SharedHome, SharedRow } from "./SharedHome";

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
  | "output_styles"
  | "sub_agents"
  | "commands"
  | "mcp_servers"
  | "allowlist"
  | "models"
  // The three the shared home carries that fleet's read does not: they are drawn only against a home.
  | "workflows"
  | "global_instructions"
  | "memory";

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
  /** Output styles only: where the style came from and whether it is in force. */
  style?: KitSetupStyle;
  /** The plugin that provides it, by its own name. Absent for your own. */
  plugin?: string;
};

export type KitSetupStyle = {
  from: "person" | "repository" | "plugin" | "built_in" | "armada";
  applied: "active" | "available" | "always";
  /** The plugin's own name, on a `plugin` style. */
  plugin?: string;
};

export type KitSetupUnreadable = {
  source: string;
  why: string;
};

const KIND_LABEL: Record<KitSetupKindWord, string> = {
  skills: "Skills",
  plugins: "Plugins",
  agent_file: "Agent file",
  output_styles: "Output styles",
  sub_agents: "Sub agents",
  commands: "Commands",
  mcp_servers: "MCP servers",
  allowlist: "Allowlist",
  models: "Models",
  workflows: "Workflows",
  global_instructions: "Global instructions",
  memory: "Memory",
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
  output_styles: "No output styles",
  sub_agents: "No sub agents",
  commands: "No commands",
  mcp_servers: "No servers connected",
  allowlist: "Nothing always-allowed",
  models: "No models named",
  workflows: "No workflows",
  global_instructions: "No global instructions",
  memory: "No memory",
};

/**
 * Against a shared home the read gains Workflows, Memory and Global instructions, the last
 * beside the agent file. The owner's decision, 13 Oct: both tiles draw.
 */
function withHomeKinds(kinds: KitSetupKind[], rows: SharedRow[]): KitSetupKind[] {
  const tile = (kind: KitSetupKindWord): KitSetupKind[] => {
    const here = rows.filter((row) => row.kind === kind);
    return here.length === 0 ? [] : [{ kind, read: { what: "read", items: [], unreadable: [] } }];
  };
  return kinds.flatMap((one) => {
    if (one.kind === "agent_file") return [one, ...tile("global_instructions")];
    if (one.kind === "commands") return [one, ...tile("workflows")];
    if (one.kind === "allowlist") return [one, ...tile("memory")];
    return [one];
  });
}

export function KitSetup({ setup, onRemoveAllowed }: KitSetupProps) {
  const shared = useSharedHome();
  // The row whose two versions are open beside the page.
  const [comparing, setComparing] = useState<SharedRow | null>(null);
  // Before the read answers there is nothing yet to say.
  if (setup === undefined) return null;
  const kinds = shared === undefined ? setup.kinds : withHomeKinds(setup.kinds, shared.rows);
  return (
    <section className="armada-kit-setup" aria-label={setup.harness}>
      <header className="armada-kit-setup__head">
        <h3 className="armada-kit-setup__title">{setup.harness}</h3>
        <p className="armada-kit-setup__where">
          <span className="armada-kit-setup__home">{setup.home}</span>
        </p>
        {shared === undefined ? null : <SharedHomeBar home={shared} />}
      </header>

      {/* The glass the kinds stand on, each a tile. */}
      <div className="armada-kit-setup__glass">
        {kinds.map((kind) => (
          <Kind key={kind.kind} home={setup.home} shared={shared} onCompare={setComparing} onRemoveAllowed={onRemoveAllowed} {...kind} />
        ))}
      </div>
      {shared === undefined || comparing === null ? null : (
        <SyncSheet row={comparing} kindLabel={KIND_LABEL[comparing.kind as KitSetupKindWord]} acts={shared} onClose={() => setComparing(null)} />
      )}
    </section>
  );
}

/** Kit's words for its kinds, for the pages that draw a kind by itself. */
export const KIT_KIND_LABEL = KIND_LABEL;
export type KitLine = Line;

/** One row as the tile draws it: the thing read, or the thing only the home has, and how it stands against the home. */
type Line = { item: KitSetupItem; row?: SharedRow | undefined };

/** What is read, then what only the home has. Where a kind has no home, every line is just what was read. */
export function linesOf(kind: KitSetupKindWord, items: KitSetupItem[], shared: SharedHome | undefined): Line[] {
  const rows = shared?.rows.filter((row) => row.kind === kind) ?? [];
  const read = items.map((item) => ({ item, row: rows.find((row) => row.name === item.name) }));
  const apart = rows
    .filter((row) => !items.some((item) => item.name === row.name))
    .map((row) => ({ item: { name: row.name, ...(row.says === undefined ? {} : { says: row.says }), source: row.source ?? "" }, row }));
  return [...read, ...apart];
}

function Kind({
  kind,
  read,
  home,
  shared,
  onCompare,
  onRemoveAllowed,
}: KitSetupKind & {
  home: string;
  shared: SharedHome | undefined;
  onCompare: (row: SharedRow) => void;
  onRemoveAllowed?: ((run: string) => void) | undefined;
}) {
  const lines = read.what === "read" ? linesOf(kind, read.items, shared) : [];
  // The allowlist draws no count: a count beside the list it counts is design-system hard rule 7.
  const counted = read.what === "read" && kind !== "allowlist" && kind !== "output_styles" && shared === undefined ? read.items.length : undefined;
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
        <Allowed lines={lines} shared={shared} onCompare={onCompare} onRemove={onRemoveAllowed} />
      ) : (
        <Items kind={kind} home={home} lines={lines} shared={shared} onCompare={onCompare} unreadable={read.unreadable} />
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
export function sourceWords(source: string): string {
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
  lines,
  shared,
  onCompare,
  onRemove,
}: {
  lines: Line[];
  shared: SharedHome | undefined;
  onCompare: (row: SharedRow) => void;
  onRemove?: ((run: string) => void) | undefined;
}) {
  if (lines.length === 0) return null;
  return (
    <Scrolling>
    <ul className="armada-kit-setup__items">
      {lines.map(({ item, row }) => (
        <li className="armada-kit-setup__item" key={item.name} data-sync={row?.state}>
          <span className="armada-kit-setup__item-name mono" title={item.name}>{item.name}</span>
          {item.source === "" ? null : <span className="armada-kit-setup__item-from">{sourceWords(item.source)}</span>}
          <Synced row={row} shared={shared} onCompare={onCompare} />
          {onRemove === undefined || row?.state === "home" ? null : (
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
    </Scrolling>
  );
}

/** Where a style came from, in words. A plugin is named by its own name. */
export function styleFrom(style: KitSetupStyle): string {
  switch (style.from) {
    case "person":
      return "Yours";
    case "repository":
      return "This repository";
    case "plugin":
      return style.plugin ?? "A plugin";
    case "built_in":
      return "Built in";
    case "armada":
      return "Armada";
  }
}

/**
 * One output style: its name, what it says, and where it came from. **The active one is marked
 * by a glyph and carried by weight**, so the choice reads without a count or a colour.
 */
function StyleRow({ item, style }: { item: KitSetupItem; style: KitSetupStyle }) {
  const active = style.applied === "active";
  return (
    <li className="armada-kit-setup__style" data-applied={style.applied}>
      <span className="armada-kit-setup__item-name">{item.name}</span>
      {active ? (
        <Tooltip beside label="Active">
          <span className="armada-kit-setup__style-mark" role="img" aria-label="Active">
            <Check size={16} strokeWidth={2} aria-hidden="true" />
          </span>
        </Tooltip>
      ) : null}
      {item.says === undefined ? null : (
        <span className="armada-kit-setup__item-says" title={item.says}>
          {item.says}
        </span>
      )}
      <span className="armada-kit-setup__style-from">{styleFrom(style)}</span>
    </li>
  );
}

/**
 * Armada's session rules, set apart from the styles. **Not a style the person picks**: it sits on
 * top of whichever one is active in the sessions Bridge hosts, so it has no mark and a heading of
 * its own.
 */
function AlwaysApplied({ rows }: { rows: { item: KitSetupItem; style: KitSetupStyle }[] }) {
  if (rows.length === 0) return null;
  return (
    <section className="armada-kit-setup__always" aria-label="Always applied">
      <h5 className="armada-kit-setup__always-name">Always applied</h5>
      <ul className="armada-kit-setup__items">
        {rows.map(({ item, style }) => (
          <StyleRow key={item.name} item={item} style={style} />
        ))}
      </ul>
    </section>
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

/**
 * A tile's list, about eight rows tall and scrolling inside the tile past that. **It is a Tab stop
 * only while it overflows**, and takes the arrows then (`data-scrolls`, `tile-grid.ts`); a list
 * that fits has nothing to scroll and nothing to stop at.
 */
function Scrolling({ children }: { children: ReactNode }) {
  const area = useRef<HTMLDivElement>(null);
  const [overflows, setOverflows] = useState(false);
  useLayoutEffect(() => {
    const element = area.current;
    if (element === null) return undefined;
    const measure = () => setOverflows(element.scrollHeight > element.clientHeight + 1);
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(element);
    return () => watch.disconnect();
  }, []);
  return (
    // `ScrollArea`'s own class, for its scrollbar drawn from the tokens, on a div this can measure.
    <div
      ref={area}
      className="armada-scroll-area armada-kit-setup__scroll"
      {...(overflows ? { tabIndex: 0, "data-scrolls": "" } : {})}
    >
      {children}
    </div>
  );
}

/** A row's marks and the act it offers, where a home is given and says anything of the row. */
function Synced({
  row,
  shared,
  onCompare,
}: {
  row: SharedRow | undefined;
  shared: SharedHome | undefined;
  onCompare: (row: SharedRow) => void;
}) {
  if (row === undefined || shared === undefined) return null;
  return (
    <>
      <SyncMarks row={row} onCompare={onCompare} />
      {row.state === "here" || row.state === "home" ? (
        <span className="armada-kit-setup__item-act">
          <SyncAct row={row} acts={shared} />
        </span>
      ) : null}
    </>
  );
}

function Items({
  kind,
  home,
  lines,
  shared,
  onCompare,
  unreadable,
}: {
  kind: KitSetupKindWord;
  home: string;
  lines: Line[];
  shared: SharedHome | undefined;
  onCompare: (row: SharedRow) => void;
  unreadable: KitSetupUnreadable[];
}) {
  // Output styles: the rules stand under the styles, set apart.
  const rules = lines.flatMap(({ item }) => (item.style?.applied === "always" ? [{ item, style: item.style }] : []));
  const picked = rules.length === 0 ? lines : lines.filter(({ item }) => item.style?.applied !== "always");
  return (
    <>
      {picked.length === 0 && unreadable.length === 0 ? (
        <p className="armada-kit-setup__none">{KIND_NONE[kind]}</p>
      ) : (
        <Scrolling>
        <ul className="armada-kit-setup__items">
          {picked.map(({ item, row }) =>
            item.style === undefined ? (
            <li className="armada-kit-setup__item" key={`${item.source}/${item.name}`} data-sync={row?.state}>
              <span className="armada-kit-setup__item-name">{item.name}</span>
              {item.says === undefined ? null : (
                /* One line, and the whole of it on hover: twenty-one rows each
                   three lines deep is a list nobody reads to the end of. */
                <span className="armada-kit-setup__item-says" title={item.says}>
                  {item.says}
                </span>
              )}
              {item.source === "" ? null : (
                <span className="armada-kit-setup__item-source" title={item.source}>
                  {under(item.source, home)}
                </span>
              )}
              <Synced row={row} shared={shared} onCompare={onCompare} />
            </li>
            ) : (
              <StyleRow key={`${item.source}/${item.name}`} item={item} style={item.style} />
            ),
          )}
        </ul>
        </Scrolling>
      )}
      <AlwaysApplied rows={rules} />
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
