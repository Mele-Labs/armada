import { Fragment, useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { Database, Folder, FolderGit2, Laptop } from "lucide-react";
import type { LucideIcon } from "lucide-react";

import { Input } from "../../primitives/Input/Input";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/** Where a repository's runs are kept. `detail` names the folder, the profile or the bucket address where the place alone does not. */
import type { RunsPlace, RunsPlaceId, SyncedFolder } from "./runs-places";
export type { RunsPlace, RunsPlaceId, SyncedFolder } from "./runs-places";

const NAME: Record<RunsPlaceId, string> = {
  mac: "On this Mac",
  repository: "In the repository",
  synced: "iCloud or Google Drive",
  cloud: "Cloud storage",
};

export const PLACE_GLYPH: Record<RunsPlaceId, LucideIcon> = { mac: Laptop, repository: FolderGit2, synced: Folder, cloud: Database };

/** Where the runs are kept as a phrase: `on this Mac`, `in iCloud Drive`. */
export function placePhrase(place: RunsPlace): string {
  if (place.id === "mac") return "on this Mac";
  if (place.id === "repository") return "in the repository";
  return place.detail === undefined ? (place.id === "synced" ? "in a synced folder" : "in cloud storage") : `in ${place.detail}`;
}

const ADDRESS = /^(s3|r2|gs):\/\/\S+$/;
const PLACES: readonly RunsPlaceId[] = ["mac", "repository", "synced", "cloud"];

/** One line a row can be: a place, or a choice under the place that asked a second question. */
type Item = { key: string; place: RunsPlaceId; sub?: "folder" | "profile" | "address"; at?: number };

/**
 * The places a repository's runs can be kept, as rows in place: asked when a repository is added, and again from the layer that lists its runs.
 * J and K move, Enter picks, Esc goes back a line and then closes.
 * Synced folders and cloud storage ask a second line before they pick.
 */
export function RunsPlaces({
  synced,
  profiles,
  current,
  onPick,
  onClose,
  seize = true,
  refused,
  onFolder,
}: {
  synced: readonly SyncedFolder[];
  /** The profiles signed in to Armada's store. */
  profiles: readonly string[];
  /** Where it is kept now, on the row it opens at. */
  current?: RunsPlace | undefined;
  onPick: (place: RunsPlace) => void;
  onClose: () => void;
  /** Takes the focus as it shows. Off, the rows are left where they stand until the focus is in them, so a card beside a message box does not hold typing. */
  seize?: boolean;
  /** Fleet's words for the pick it would not take. The place stays where it was. */
  refused?: string | undefined;
  /** Asks for a folder on this Mac, for a synced place that no detected folder names. Answers its path, or nothing if none was chosen. */
  onFolder?: (() => Promise<string | null>) | undefined;
}) {
  const [open, setOpen] = useState<RunsPlaceId | null>(null);
  const [at, setAt] = useState(() => Math.max(0, PLACES.indexOf(current?.id ?? "mac")));
  const [address, setAddress] = useState("");
  const root = useRef<HTMLDivElement>(null);

  const items: Item[] = PLACES.flatMap((place): Item[] => {
    const head: Item = { key: place, place };
    if (open !== place) return [head];
    if (place === "synced") return [head, ...synced.map((_, index): Item => ({ key: `synced:${index}`, place, sub: "folder", at: index }))];
    return [head, ...profiles.map((_, index): Item => ({ key: `cloud:${index}`, place, sub: "profile", at: index })), { key: "cloud:address", place, sub: "address" }];
  });
  const active = Math.min(at, items.length - 1);

  useEffect(() => {
    if (!seize && !root.current?.contains(document.activeElement)) return;
    root.current?.querySelector<HTMLElement>(`[data-item="${items[active]?.key}"]`)?.focus();
    // The focus follows the row that is active, whatever moved it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, open]);

  const line: Record<RunsPlaceId, string> = {
    mac: "Kept in your Kit. Nothing in the repository.",
    repository: "armada.yml written on the Job's branch, landing with its pull request.",
    synced: synced.length === 0 ? "A folder a service keeps in sync" : synced.length === 1 ? synced[0]!.path : synced.map((one) => one.label).join(", "),
    cloud: "Armada's store, or a bucket address",
  };

  const valid = ADDRESS.test(address.trim());
  function choose(index: number): void {
    const item = items[index];
    if (item === undefined) return;
    setAt(index);
    if (item.sub === "folder") return onPick({ id: "synced", detail: synced[item.at!]!.label });
    if (item.sub === "profile") return onPick({ id: "cloud", detail: `Armada's store, ${profiles[item.at!]!}` });
    if (item.sub === "address") return valid ? onPick({ id: "cloud", detail: address.trim() }) : undefined;
    if (item.place === "synced" && synced.length === 0 && onFolder !== undefined) {
      return void onFolder().then((path) => (path === null ? undefined : onPick({ id: "synced", detail: path })));
    }
    if (item.place === "synced" && synced.length === 1) return onPick({ id: "synced", detail: synced[0]!.label });
    if (item.place === "synced" || item.place === "cloud") {
      setOpen(item.place);
      return setAt(index + 1);
    }
    onPick({ id: item.place });
  }

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const typing = event.target instanceof HTMLInputElement;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      if (open === null) return onClose();
      setAt(PLACES.indexOf(open));
      return setOpen(null);
    }
    const down = event.key === "ArrowDown" || (!typing && event.key === "j");
    const up = event.key === "ArrowUp" || (!typing && event.key === "k");
    if (down || up) {
      event.preventDefault();
      event.stopPropagation();
      return setAt((active + (down ? 1 : items.length - 1)) % items.length);
    }
    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      choose(active);
    }
  };

  /** A row's own text: the place's name and the one line that says what happens. */
  const words = (name: string, said: string, mono: boolean) => (
    <>
      <span className="armada-runs__place-name">{name}</span>
      <span className="armada-runs__place-line" data-mono={mono || undefined}>
        {said}
      </span>
    </>
  );

  /** A choice under a place that asked a second line. */
  const sub = (key: string, name: string, said: string, mono: boolean) => {
    const index = items.findIndex((one) => one.key === key);
    return (
      <button key={key} type="button" className="armada-runs__place" data-item={key} data-active={active === index || undefined} onClick={() => choose(index)} onMouseMove={() => setAt(index)}>
        {words(name, said, mono)}
      </button>
    );
  };

  return (
    <div ref={root} role="group" aria-label="Where Armada configuration is kept" className="armada-runs__places" onKeyDown={keys}>
      {PLACES.map((place) => {
        const index = items.findIndex((one) => one.key === place);
        const Glyph = PLACE_GLYPH[place];
        return (
          <Fragment key={place}>
            <button
              type="button"
              className="armada-runs__place"
              data-item={place}
              data-active={active === index || undefined}
              aria-current={current?.id === place || undefined}
              aria-expanded={place === "synced" || place === "cloud" ? open === place : undefined}
              onClick={() => choose(index)}
              onMouseMove={() => setAt(index)}
            >
              <Tooltip label={NAME[place]}>
                <span className="armada-runs__place-mark">
                  <Glyph size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
              {words(NAME[place], line[place], place === "synced" && synced.length === 1)}
            </button>
            {open !== place ? null : (
              <div className="armada-runs__sub">
                {place === "synced" ? synced.map((one, at) => sub(`synced:${at}`, one.label, one.path, true)) : null}
                {place === "cloud" ? profiles.map((one, at) => sub(`cloud:${at}`, one, "Armada's store", false)) : null}
                {place !== "cloud" ? null : (
                  <div className="armada-runs__address" data-active={items[active]?.key === "cloud:address" || undefined}>
                    <Input
                      aria-label="Bucket address"
                      data-item="cloud:address"
                      mono
                      placeholder="s3://bucket/path"
                      value={address}
                      onChange={(event) => setAddress(event.target.value)}
                      onFocus={() => setAt(items.findIndex((it) => it.key === "cloud:address"))}
                    />
                  </div>
                )}
              </div>
            )}
          </Fragment>
        );
      })}
      {refused === undefined ? null : (
        <p role="alert" className="armada-runs__refused">
          {refused}
        </p>
      )}
    </div>
  );
}
