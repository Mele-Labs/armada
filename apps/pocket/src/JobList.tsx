// The Jobs list as native draws it (`crates/bridge-native/src/jobs.rs`): a heading per project, and per Job a card with
// its lamp on the leading edge, its title, the last turn's age, its Mac's dot and its state's glyph. A Job another
// started folds under it; folded, its starter carries one glyph per state its sub-Jobs are in.

import type { CSSProperties } from "react";
import { ChevronDown, ChevronRight, Link as Linked, PowerOff, Unplug } from "lucide-react";
import {
  bucketOf,
  Button,
  sessionGlyphOf,
  Stamp,
  Tooltip,
  type SessionState,
} from "@armada/components";

import type { Item, Line, Mac } from "./fleet";
import { lampOf, standingOf } from "./rowState";
import { MAC_HUES } from "./Notch";

/** The words a state's glyph says, as native's `glyph_of` gives them. */
const WORDS: Record<SessionState, string> = {
  working: "Working",
  running: "Running",
  subagent: "Subagent working",
  waiting: "Waiting on you",
  stopped_asks: "Waiting on you",
  done: "Done",
  stuck: "Stuck",
  failing: "Checks failed",
  unstarted: "Not started",
  stopped: "Stopped",
  held: "Held",
  limited: "Held for the usage limit",
  after: "Starts after another Job",
  draft: "Reading the request",
  blank: "Blank: no slot, no branch",
  idle: "Idle",
  replied: "Unread reply",
  quiet: "Not reachable",
  ended: "Ended",
  paused: "Paused",
};

/** A state's glyph in its bucket's hue, named by its tooltip. */
export function StateGlyph({
  state,
  said,
  size = 16,
}: {
  state: SessionState;
  said?: string;
  size?: number;
}) {
  const Glyph = sessionGlyphOf(state);
  const words = said ?? WORDS[state];
  return (
    <Tooltip label={words}>
      <span
        className="pk-glyph"
        data-bucket={state === "paused" ? "idle" : bucketOf(state)}
        role="img"
        aria-label={words}
      >
        <Glyph size={size} aria-hidden="true" />
      </span>
    </Tooltip>
  );
}

/** A Mac's dot in its hue, named by its tooltip. */
export function MacDot({ mac, macs }: { mac: Mac; macs: readonly Mac[] }) {
  const at = Math.max(0, macs.indexOf(mac));
  const name = mac.machine.name || new URL(mac.machine.origin).hostname;
  return (
    <Tooltip label={name}>
      <span
        className="pk-mac"
        data-hue={MAC_HUES[at % MAC_HUES.length]}
        role="img"
        aria-label={name}
      />
    </Tooltip>
  );
}

/** What a row's second line says, where it has one. */
function secondOf(item: Item): {
  chips: string[];
  says: string;
} {
  const { session, state, said } = item;
  if (state === "done")
    return {
      chips: [],
      says: session.done_summary ?? "",
    };
  if (bucketOf(state) === "waiting" || state === "unstarted" || state === "stopped_asks")
    return {
      chips: [],
      says: said,
    };
  const chips = [
    ...(session.pull === undefined
      ? []
      : [
          `#${session.pull}`,
        ]),
    ...(session.issue === undefined
      ? []
      : [
          session.issue.number,
        ]),
  ];
  return {
    chips,
    says: [
      session.step,
      session.now,
    ]
      .filter((one) => one !== undefined && one !== "")
      .join(" · "),
  };
}

/** The glyphs a folded starter carries: one per state its sub-Jobs are in, Needs you first, each naming them. */
function Strip({ kids }: { kids: readonly Item[] }) {
  const states: SessionState[] = [];
  for (const kid of [
    ...kids,
  ].sort((a, b) => standingOf(a)[0] - standingOf(b)[0]))
    if (!states.includes(kid.state)) states.push(kid.state);
  return (
    <span className="pk-strip">
      {states.map((state) => (
        <StateGlyph
          key={state}
          state={state}
          size={12}
          said={`${WORDS[state]}: ${kids
            .filter((kid) => kid.state === state)
            .map((kid) => kid.session.title ?? "")
            .join(", ")}`}
        />
      ))}
    </span>
  );
}

export function JobRow({
  line,
  macs,
  several,
  kids,
  folded,
  onFold,
  onOpen,
  onEnd,
}: {
  line: Line;
  macs: readonly Mac[];
  several: boolean;
  /** Its sub-Jobs, where it started any. */
  kids: readonly Item[];
  folded: boolean;
  onFold: () => void;
  onOpen: () => void;
  onEnd?: () => void;
}) {
  const { item, depth, through, last } = line;
  const { session, state, said } = item;
  const lamp = lampOf(item);
  const { chips, says } = secondOf(item);
  const title = session.title ?? "";
  const glyphSaid =
    session.mark === undefined && state !== "done" && state !== "idle" ? said : WORDS[state];
  return (
    <li
      className="pk-job"
      data-depth={depth > 0 || undefined}
      style={
        depth > 0
          ? ({
              "--pk-depth": depth,
            } as CSSProperties)
          : undefined
      }
    >
      {depth === 0 ? null : (
        <span className="pk-job__guides" aria-hidden="true">
          {through.map((runs, at) =>
            runs ? (
              <span
                key={at}
                className="pk-job__guide"
                style={
                  {
                    "--pk-column": at,
                  } as CSSProperties
                }
              />
            ) : null,
          )}
          {last ? null : (
            <span
              className="pk-job__guide"
              style={
                {
                  "--pk-column": depth - 1,
                } as CSSProperties
              }
            />
          )}
          <span
            className="pk-job__guide pk-job__guide--elbow"
            style={
              {
                "--pk-column": depth - 1,
              } as CSSProperties
            }
          />
        </span>
      )}
      <div className="pk-job__card" data-lamp={lamp} data-sub={depth > 0 || undefined}>
        <span className="pk-job__fold">
          {kids.length === 0 ? null : (
            <Tooltip label={folded ? "Unfold sub-Jobs" : "Fold sub-Jobs"}>
              <button
                type="button"
                className="pk-job__chevron"
                aria-label={folded ? "Unfold sub-Jobs" : "Fold sub-Jobs"}
                aria-expanded={!folded}
                onClick={onFold}
              >
                {folded ? (
                  <ChevronRight size={14} aria-hidden="true" />
                ) : (
                  <ChevronDown size={14} aria-hidden="true" />
                )}
              </button>
            </Tooltip>
          )}
        </span>
        <button type="button" className="pk-job__open" aria-label={title} onClick={onOpen}>
          <span className="pk-job__line">
            {depth === 0 || session.from === undefined ? null : (
              <Tooltip label={`Linked Job, started by ${session.from.title}`}>
                <span
                  className="pk-job__link"
                  role="img"
                  aria-label={`Started by ${session.from.title}`}
                >
                  <Linked size={12} aria-hidden="true" />
                </span>
              </Tooltip>
            )}
            <span className="pk-job__title">{title}</span>
            {!folded || kids.length === 0 ? null : <Strip kids={kids} />}
            {session.last_turn_at === undefined ? null : (
              <Tooltip label="Last turn">
                <span className="pk-job__time">
                  <Stamp still first="age" at={session.last_turn_at} />
                </span>
              </Tooltip>
            )}
            {several ? <MacDot mac={item.mac} macs={macs} /> : null}
            <StateGlyph state={state} said={glyphSaid} />
          </span>
          {chips.length === 0 && says === "" ? null : (
            <span className="pk-job__second">
              {chips.map((chip) => (
                <span key={chip} className="pk-job__chip">
                  {chip}
                </span>
              ))}
              {says === "" ? null : <span className="pk-job__says">{says}</span>}
            </span>
          )}
        </button>
        {onEnd === undefined ? null : (
          <Tooltip label="End job">
            <Button variant="ghost" size="sm" iconOnly aria-label="End job" onClick={onEnd}>
              <PowerOff size={16} aria-hidden="true" />
            </Button>
          </Tooltip>
        )}
      </div>
    </li>
  );
}

/** A Mac that forgot the phone or is not answering: a row of its own, failed, over the list. */
export function MacRow({ mac, onPair }: { mac: Mac; onPair: () => void }) {
  const name = mac.machine.name || new URL(mac.machine.origin).hostname;
  const said = mac.paired ? "Not answering" : "Not paired";
  return (
    <li className="pk-job">
      <div className="pk-job__card" data-lamp="failed">
        <span className="pk-job__fold" />
        <div className="pk-job__open">
          <span className="pk-job__line">
            <span className="pk-job__title">{name}</span>
            <Tooltip label={said}>
              <span className="pk-glyph" data-bucket="idle" role="img" aria-label={said}>
                <Unplug size={16} aria-hidden="true" />
              </span>
            </Tooltip>
          </span>
          <span className="pk-job__second">
            <span className="pk-job__says">{mac.paired ? mac.error : said}</span>
          </span>
        </div>
        {mac.paired ? null : (
          <Button variant="secondary" size="sm" onClick={onPair}>
            Pair again
          </Button>
        )}
      </div>
    </li>
  );
}
