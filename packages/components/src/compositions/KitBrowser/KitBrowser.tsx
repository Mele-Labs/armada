import { useState, type ReactNode } from "react";
import { Check } from "lucide-react";

import { useKept } from "../../keep";
import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { KitServerAddForm, KIT_HERE_LABEL } from "../KitServers/KitServers";
import type { KitServerKind, KitServerRowProps } from "../KitServers/KitServers";
import { KIT_KIND_LABEL } from "../KitSetup/KitSetup";
import type { KitSetupKindWord, KitSetupRead } from "../KitSetup/KitSetup";
import { SharedHomeBar, SyncAct, SyncMarks, SyncSheet, useSharedHome } from "../KitSetup/SharedHome";
import type { SharedRow } from "../KitSetup/SharedHome";
import { holds, rowsOf } from "./kit-rows";
import type { KitKindId, KitRef, KitRow } from "./kit-rows";

/**
 * One kind of what a person has, as a list with the picked item beside it. **The list is the kind and
 * the right-hand column is one item**: its name, where it comes from, its path, its own words in
 * full and the two ways to reach its file.
 *
 * A kind that was read and holds nothing is an empty list. A kind that was not read says so, with
 * the reason, and is never drawn as empty.
 */
export type KitBrowserProps = {
  kind: KitKindId;
  /** The kind's own name, for the list's label. */
  label: string;
  /** `undefined` before the read lands. */
  setup: KitSetupRead | undefined;
  /** What this page's fields are kept under. */
  keep: string;
  /** The sidebar's search, narrowing the list when it is what kept this kind in view. Empty narrows nothing. */
  narrow: string;
  /** The row picked, by id. */
  picked: string | undefined;
  onPick: (id: string | undefined) => void;
  /** A row leading to an item of another kind. */
  onJump: (to: KitRef) => void;
  onRemoveAllowed?: (run: string) => void;
  /** Open the file an item opens as, in the side panel. */
  onOpen?: (path: string) => void;
  /** An item that is a folder with no single file to open. */
  onShowInFinder?: (path: string) => void;
  /** Why the last Open or Show in Finder did nothing, in the row's voice. */
  openRefused?: ReactNode;
  /** Servers: Kit's own, beside the ones connected outside Armada. */
  servers?: {
    rows: readonly KitServerRowProps[] | undefined;
    /** The repository "here" means. `null` draws no Here control. */
    here: string | null;
    refused?: ReactNode;
    onAdd?: (adding: { name: string; kind: KitServerKind; address: string }) => void;
    onForget?: (name: string) => void;
    onKitReach?: (name: string, reaches: boolean) => void;
    onHereReach?: (name: string, reach: "extended" | "restricted" | null) => void;
  };
};

export function KitBrowser({
  kind,
  label,
  setup,
  keep,
  narrow,
  picked,
  onPick,
  onJump,
  onRemoveAllowed,
  onOpen,
  onShowInFinder,
  openRefused,
  servers,
}: KitBrowserProps) {
  const shared = useSharedHome();
  const [filter, setFilter] = useKept(`${keep}:${kind}:filter`, "");
  const [adding, setAdding] = useState(false);
  const [comparing, setComparing] = useState<SharedRow | null>(null);

  const read = rowsOf(kind, setup, shared, servers?.rows);
  const rows = read.rows.filter((row) => holds(row.name, filter) && holds(row.name, narrow));
  // With nothing picked the detail shows the list's first item, so the page never draws a blank pane.
  const row = adding ? undefined : (rows.find((one) => one.id === picked) ?? rows[0]);
  const ordinary = rows.filter((one) => one.always !== true);
  const always = rows.filter((one) => one.always === true);

  const item = (one: KitRow) => (
    <li key={one.id} className="armada-kit-browser__item">
      <button
        type="button"
        className="armada-kit-browser__row"
        data-nav-item=""
        aria-current={one.id === row?.id ? "true" : undefined}
        tabIndex={one.id === row?.id ? 0 : -1}
        onClick={() => {
          setAdding(false);
          onPick(one.id);
        }}
      >
        <span className="armada-kit-browser__name" data-mono={one.mono === true ? "" : undefined}>
          {one.name}
        </span>
        <span className="armada-kit-browser__side">
          {one.active === true ? (
            <Tooltip beside label="Active">
              <span className="armada-kit-browser__active" role="img" aria-label="Active">
                <Check size={16} strokeWidth={2} aria-hidden="true" />
              </span>
            </Tooltip>
          ) : null}
          {one.meta === undefined ? null : one.metaSaid === undefined ? (
            <span className="armada-kit-browser__meta">{one.meta}</span>
          ) : (
            <Tooltip beside label={one.metaSaid}>
              <span className="armada-kit-browser__meta">{one.meta}</span>
            </Tooltip>
          )}
          {one.mark === undefined ? null : <span className="armada-kit-browser__mark">{one.mark}</span>}
        </span>
      </button>
      {one.sync === undefined || shared === undefined ? null : (
        <span className="armada-kit-browser__sync">
          <SyncMarks row={one.sync} onCompare={setComparing} />
          {one.sync.state === "here" || one.sync.state === "home" ? <SyncAct row={one.sync} acts={shared} /> : null}
        </span>
      )}
    </li>
  );

  return (
    <div className="armada-kit-browser">
      <section className="armada-kit-browser__list" aria-label={label} data-col="list">
        {shared === undefined ? null : <SharedHomeBar home={shared} />}
        <Input
          type="search"
          aria-label={`Filter ${label}`}
          placeholder="Filter"
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Escape" && filter !== "") {
              event.preventDefault();
              event.stopPropagation();
              setFilter("");
            }
          }}
        />
        {read.notRead !== undefined && read.rows.length === 0 ? (
          <p className="armada-kit-browser__not-read">
            <span className="armada-kit-browser__not-read-head">Not read</span>
            <span>{read.notRead}</span>
          </p>
        ) : (
          <div className="armada-scroll-area armada-kit-browser__scroll">
            <ul className="armada-kit-browser__items">{ordinary.map(item)}</ul>
            {always.length === 0 ? null : (
              <section aria-label="Always applied">
                <h4 className="armada-kit-browser__caption" aria-hidden="true">
                  Always applied
                </h4>
                <ul className="armada-kit-browser__items">{always.map(item)}</ul>
              </section>
            )}
          </div>
        )}
        {kind === "servers" && servers !== undefined ? (
          <div className="armada-kit-browser__foot">
            <Button
              variant="secondary"
              size="sm"
              ground="card"
              data-nav-item=""
              aria-current={adding ? "true" : undefined}
              onClick={() => {
                setAdding(true);
                onPick(undefined);
              }}
            >
              Add server
            </Button>
          </div>
        ) : null}
      </section>

      <section className="armada-kit-browser__detail" aria-label="Detail" data-col="detail">
        {adding && servers !== undefined ? (
          <div className="armada-kit-browser__form">
            <h3 className="armada-kit-browser__title">Add server</h3>
            {servers.refused === undefined ? null : <p className="armada-kit-browser__refused">{servers.refused}</p>}
            <KitServerAddForm
              keep={`${keep}:add-server`}
              onAdd={(added) => {
                servers.onAdd?.(added);
                setAdding(false);
                onPick(`servers:kit:${added.name}`);
              }}
            />
          </div>
        ) : row === undefined ? null : (
          <Detail
            row={row}
            servers={servers}
            onJump={onJump}
            onRemoveAllowed={onRemoveAllowed}
            onOpen={onOpen}
            onShowInFinder={onShowInFinder}
            openRefused={openRefused}
          />
        )}
      </section>

      {shared === undefined || comparing === null ? null : (
        <SyncSheet row={comparing} kindLabel={KIT_KIND_LABEL[comparing.kind as KitSetupKindWord] ?? comparing.kind} acts={shared} onClose={() => setComparing(null)} />
      )}
    </div>
  );
}

function Detail({
  row,
  servers,
  onJump,
  onRemoveAllowed,
  onOpen,
  onShowInFinder,
  openRefused,
}: {
  row: KitRow;
  servers: KitBrowserProps["servers"];
  onJump: (to: KitRef) => void;
  onRemoveAllowed: KitBrowserProps["onRemoveAllowed"];
  onOpen: KitBrowserProps["onOpen"];
  onShowInFinder: KitBrowserProps["onShowInFinder"];
  openRefused: KitBrowserProps["openRefused"];
}) {
  const kit = row.server?.tier === "kit" ? row.server.row : undefined;
  return (
    <article className="armada-kit-browser__item-detail" aria-label={row.name}>
      <header className="armada-kit-browser__head">
        <h3 className="armada-kit-browser__title" data-mono={row.mono === true && kit === undefined ? "" : undefined}>
          {row.name}
        </h3>
        {row.mark === undefined ? null : <span className="armada-kit-browser__mark">{row.mark}</span>}
        {row.meta === undefined ? null : row.metaSaid === undefined ? (
          <span className="armada-kit-browser__meta">{row.meta}</span>
        ) : (
          <Tooltip beside label={row.metaSaid}>
            <span className="armada-kit-browser__meta">{row.meta}</span>
          </Tooltip>
        )}
        {row.marketplace === undefined ? null : (
          <Tooltip beside label="Marketplace">
            <span className="armada-kit-browser__meta">{row.marketplace}</span>
          </Tooltip>
        )}
      </header>

      {row.path === undefined ? null : <p className="armada-kit-browser__path">{row.path}</p>}
      {row.heading === undefined ? null : <p className="armada-kit-browser__heading">{row.heading}</p>}
      {row.says === undefined ? null : (
        <p className="armada-kit-browser__says" data-mono={row.mono === true || row.server !== undefined ? "" : undefined}>
          {row.says}
        </p>
      )}

      {row.brings === undefined || row.brings.length === 0 ? null : (
        <>
          <h4 className="armada-kit-browser__caption">Brings</h4>
          <ul className="armada-kit-browser__items">
            {row.brings.map((one) => (
              <li key={one.id} className="armada-kit-browser__item">
                <button type="button" className="armada-kit-browser__row" data-nav-item="" onClick={() => onJump(one)}>
                  <span className="armada-kit-browser__name">{one.name}</span>
                  <span className="armada-kit-browser__side">
                    <span className="armada-kit-browser__mark">{one.word}</span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        </>
      )}

      {kit === undefined || servers === undefined ? null : (
        <dl className="armada-kit-browser__facts">
          <div>
            <dt>In Kit</dt>
            <dd>
              <Select aria-label={`In Kit: ${kit.name}`} value={kit.reachesByDefault ? "yes" : "no"} onChange={(event) => servers.onKitReach?.(kit.name, event.target.value === "yes")}>
                <option value="no">Off</option>
                <option value="yes">On everywhere</option>
              </Select>
            </dd>
          </div>
          {servers.here === null ? null : (
            <div>
              <dt>{servers.here}</dt>
              <dd>
                <Select
                  aria-label={`Here: ${kit.name}`}
                  value={kit.here ?? "kit"}
                  onChange={(event) => servers.onHereReach?.(kit.name, event.target.value === "kit" ? null : (event.target.value as "extended" | "restricted"))}
                >
                  <option value="kit">{KIT_HERE_LABEL.kit}</option>
                  <option value="extended">{KIT_HERE_LABEL.extended}</option>
                  <option value="restricted">{KIT_HERE_LABEL.restricted}</option>
                </Select>
              </dd>
            </div>
          )}
          <div>
            <dt>A Drone here</dt>
            <dd>{kit.resolves ? "Gets it" : "Does not"}</dd>
          </div>
        </dl>
      )}

      <div className="armada-kit-browser__acts">
        {row.opens !== undefined ? (
          <Button variant="secondary" size="sm" ground="card" onClick={() => onOpen?.(row.opens!)}>
            Open
          </Button>
        ) : row.path === undefined ? null : (
          <Button variant="secondary" size="sm" ground="card" onClick={() => onShowInFinder?.(row.path!)}>
            Show in Finder
          </Button>
        )}
        {row.removable !== true || onRemoveAllowed === undefined || row.sync?.state === "home" ? null : (
          <Button variant="secondary" size="sm" ground="card" aria-label={`Remove ${row.name}`} onClick={() => onRemoveAllowed(row.name)}>
            Remove
          </Button>
        )}
        {kit === undefined || servers?.onForget === undefined ? null : (
          <Button variant="secondary" size="sm" ground="card" aria-label={`Remove ${kit.name}`} onClick={() => servers.onForget?.(kit.name)}>
            Remove
          </Button>
        )}
      </div>
      {openRefused === undefined || row.path === undefined ? null : <p className="armada-kit-browser__refused">{openRefused}</p>}
    </article>
  );
}
