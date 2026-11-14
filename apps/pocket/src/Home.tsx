// Home: every paired Mac's live Jobs in one list, drawn as native's Jobs list: under the notch bar, a search and a scope,
// then a heading per project and its Jobs, Needs you first, each Job another started folded under its starter.
// A Mac that forgot the phone, or is not answering, is a row of its own at the top.

import { useRef, useState } from "react";
import { ChevronDown, Search } from "lucide-react";
import { bucketOf } from "@armada/components";

import { actsOn } from "./jobState";
import { homeOf, useFleet, type Item, type Line } from "./fleet";
import { JobRow, MacRow } from "./JobList";
import { Notch } from "./Notch";
import { draftPath, go, jobPath, pairPath } from "./router";

/** Which Jobs the list holds, as native's scope does; the phone has no ended Jobs to show. */
const SCOPES = {
  needs: "Needs you",
  active: "Active",
  paused: "Paused",
  drafts: "Drafts",
  all: "All",
} as const;
type Scope = keyof typeof SCOPES;

const inScope = (scope: Scope, item: Item): boolean => {
  const draft = item.session.held || item.session.blank === true;
  switch (scope) {
    case "needs":
      return bucketOf(item.state) === "needs" && item.state !== "paused";
    case "active":
      return item.state !== "paused";
    case "paused":
      return item.state === "paused";
    case "drafts":
      return draft;
    case "all":
      return true;
  }
};

const FOLDS = "pocket.folded";
const keyOf = (item: Item) => `${item.mac.machine.origin} ${item.session.id}`;
const keptFolds = (): Set<string> => {
  try {
    return new Set(JSON.parse(localStorage.getItem(FOLDS) ?? "[]") as string[]);
  } catch {
    return new Set();
  }
};

/**
 * The lines a group draws: a folded starter's sub-Jobs leave the list and ride on it as glyphs. Active leaves out a
 * paused Job, as native's does, unless a sub-Job of it is not paused, which keeps its starter in view.
 */
function shownOf(lines: readonly Line[], folded: ReadonlySet<string>) {
  const shown: {
    line: Line;
    kids: Item[];
  }[] = [];
  let under = Number.POSITIVE_INFINITY;
  lines.forEach((line, at) => {
    if (line.depth > under) return;
    under = Number.POSITIVE_INFINITY;
    const kids: Item[] = [];
    for (const next of lines.slice(at + 1)) {
      if (next.depth <= line.depth) break;
      kids.push(next.item);
    }
    if (
      [
        line.item,
        ...kids,
      ].every((one) => one.state === "paused")
    ) {
      under = line.depth;
      return;
    }
    if (kids.length > 0 && folded.has(keyOf(line.item))) under = line.depth;
    shown.push({
      line,
      kids,
    });
  });
  return shown;
}

export function Home() {
  const fleet = useFleet();
  const { several, repositories, troubled } = homeOf(fleet);
  const [scope, setScope] = useState<Scope>("active");
  const [words, setWords] = useState("");
  const [project, setProject] = useState<string | undefined>(undefined);
  const [folded, setFolded] = useState(keptFolds);
  const search = useRef<HTMLInputElement>(null);
  const fold = (item: Item) => {
    const next = new Set(folded);
    if (!next.delete(keyOf(item))) next.add(keyOf(item));
    localStorage.setItem(
      FOLDS,
      JSON.stringify([
        ...next,
      ]),
    );
    setFolded(next);
  };
  /** A search or a narrower scope lists the Jobs it finds on their own, out of their starters' folds. */
  const narrowed = scope !== "active" || words.trim() !== "";
  const match = (item: Item) =>
    inScope(scope, item) &&
    (item.session.title ?? "").toLowerCase().includes(words.trim().toLowerCase());
  const groups = repositories
    .filter((one) => project === undefined || one.name === project)
    .map((one) => ({
      name: one.name,
      shown: narrowed
        ? one.lines
            .filter((line) => match(line.item))
            .map((line) => ({
              line: {
                ...line,
                depth: 0,
                through: [],
                last: true,
              },
              kids: [],
            }))
        : shownOf(one.lines, folded),
    }))
    .filter((one) => one.shown.length > 0);
  return (
    <section className="pk-screen" aria-label="Home">
      <Notch
        macs={fleet.macs}
        at={fleet.at}
        place="jobs"
        onSearch={() => search.current?.focus()}
        projects={{
          names: repositories.map((one) => one.name),
          shown: project,
          onShow: setProject,
          onDrafts: () => setScope("drafts"),
        }}
      />
      <div className="pk-tools">
        <label className="pk-search">
          <Search size={14} aria-hidden="true" />
          <input
            ref={search}
            type="search"
            placeholder="Search Jobs"
            aria-label="Search Jobs"
            value={words}
            onChange={(event) => setWords(event.target.value)}
          />
        </label>
        <label className="pk-scope">
          <select
            aria-label="Which Jobs the list holds"
            value={scope}
            onChange={(event) => setScope(event.target.value as Scope)}
          >
            {Object.entries(SCOPES).map(([value, name]) => (
              <option key={value} value={value}>
                {name}
              </option>
            ))}
          </select>
          <ChevronDown size={14} aria-hidden="true" />
        </label>
      </div>
      <div className="pk-body pk-home">
        {troubled.length === 0 ? null : (
          <ul className="pk-jobs" aria-label="Macs">
            {troubled.map((mac) => (
              <MacRow
                key={mac.machine.origin}
                mac={mac}
                onPair={() => go(pairPath(mac.machine.origin))}
              />
            ))}
          </ul>
        )}
        {groups.map((group) => (
          <section key={group.name} className="pk-group" aria-label={group.name}>
            <h2 className="pk-group__head">{group.name}</h2>
            <ul className="pk-jobs">
              {group.shown.map(({ line, kids }) => {
                const { session } = line.item;
                const origin = line.item.mac.machine.origin;
                return (
                  <JobRow
                    key={keyOf(line.item)}
                    line={line}
                    macs={fleet.macs}
                    several={several}
                    kids={kids}
                    folded={folded.has(keyOf(line.item))}
                    onFold={() => fold(line.item)}
                    onOpen={() => go((session.held ? draftPath : jobPath)(origin, session.id))}
                    {...(line.item.state === "done"
                      ? {
                          onEnd: () =>
                            void actsOn(line.item.mac.machine, () => undefined)
                              .close({
                                session_id: session.id,
                              })
                              .catch(() => undefined),
                        }
                      : {})}
                  />
                );
              })}
            </ul>
          </section>
        ))}
      </div>
    </section>
  );
}
