import { createContext, useContext } from "react";
import type { ReactNode } from "react";
import { Laptop, Monitor, SquareTerminal } from "lucide-react";

import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Who a pull request was registered to, read off the central store keyed by the
 * person's GitHub login. **A row holds a kind, a short id and a machine's name,
 * and nothing about the pull request**: no title, no repository, no code. The
 * title is drawn only where this machine's own ledger knows it.
 */
export type RegisteredOwnerRow = {
  kind: "session";
  /** The short id, as the machine that claimed it names it. */
  id: string;
  machine: { name: string; here: boolean };
};

/** What the window holds of the registry. Absent, nothing is registered and nothing is drawn. */
export type PrOwnerRegistry = {
  /** The row for a pull request. Absent where its repository registers nothing, or no row exists. */
  ownerOf: (repository: string, number: number) => RegisteredOwnerRow | undefined;
  /** Whether the repository whose Manifest is open registers owners. */
  registering: boolean;
  setRegistering: (on: boolean) => void;
};

const Held = createContext<PrOwnerRegistry | undefined>(undefined);

export function PrOwnerRegistryProvider({ value, children }: { value: PrOwnerRegistry; children: ReactNode }) {
  return <Held.Provider value={value}>{children}</Held.Provider>;
}

export function usePrOwnerRegistry(): PrOwnerRegistry | undefined {
  return useContext(Held);
}

/** `owner/name`, where an address is a forge's `/<owner>/<name>/pull/<number>`. */
export function repositoryOfAddress(address: string): string | undefined {
  try {
    return /^\/([^/]+\/[^/]+)\/pull\/\d+/.exec(new URL(address).pathname)?.[1];
  } catch {
    return undefined;
  }
}

const KIND = {
  session: { Glyph: SquareTerminal, said: "Job" },
} as const;

/**
 * The owner of a pull request as a chip: the kind's glyph and the short id in
 * mono, then the machine's mark past a divider, as a pull request's Checks end
 * the reference chip. **Marks with tooltips, never a phrase.** `laptop` is this
 * machine; `monitor` is another one, and its tooltip names it.
 */
export function RegisteredOwner({ row }: { row: RegisteredOwnerRow }) {
  const { Glyph, said } = KIND[row.kind];
  const Machine = row.machine.here ? Laptop : Monitor;
  const machine = row.machine.here ? "This machine" : row.machine.name;
  return (
    <span className="armada-registered-owner" role="group" aria-label={`${said} ${row.id}, ${machine}`}>
      <Tooltip label={said}>
        <span className="armada-registered-owner__who">
          <Glyph size={12} strokeWidth={2} aria-hidden />
          <span>{row.id}</span>
        </span>
      </Tooltip>
      <Tooltip label={machine}>
        <span className="armada-registered-owner__machine" data-here={row.machine.here || undefined} role="img" aria-label={machine}>
          <Machine size={12} strokeWidth={2} aria-hidden />
        </span>
      </Tooltip>
    </span>
  );
}

/** The chip for a pull request, where the window holds a row for it. Nothing otherwise. */
export function RegisteredOwnerOf({ repository, number }: { repository: string | undefined; number: number }) {
  const row = usePrOwnerRegistry()?.ownerOf(repository ?? "", number);
  return row === undefined ? null : <RegisteredOwner row={row} />;
}
