// The flight controls' tiles while a Session is open on the stage: the same block, size and keys, `1` to `5`, as the six usual
// ones, with the Session's own acts. What each does and why one is off is in `session-tiles.ts`. A refusal from Fleet is the
// toast the Session's composer shows, since the same `draft.send` carries it. **Report** opens a dialog over the window (`ReportBox`);
// sending starts a Session in Armada's repository where Fleet serves it (`reportRoot`), tells it the owner's words and what Bridge holds about this one
// (`report-brief.ts`), and opens it on the stage. The owner's ask of 15 Oct 2026. **Release** opens a confirmation naming what the Session
// holds, and what Fleet answers stands in place of the tiles until Close (`ReleaseHolds.tsx`).
//
// **Which tiles are drawn, in what order, with what name, glyph and key is the owner's** (`flight-controls.ts`, edited in place by
// `FlightEdit.tsx`); this file draws what that says and does what each tile does. Facts live on the tiles: Compact carries the context
// window's fill, Auto land and Pause are lit while on and paused, Checks pulses while any Check runs, Subagents shows how many are running.
// A Shell action runs through Fleet in the Job's slot, and its output stands in place of the tiles until Close (`ShellBox.tsx`). The owner's ask of 18 Oct 2026.
//
// **A shell tile the repository committed asks once before it runs on this Mac**, showing its command, and asks again when the command
// changes (`TileAskDialog`). A tile of the person's that hides the repository's carries the eclipse mark naming what it hides.

import { useEffect, useRef, useState, type CSSProperties } from "react";
import { Circle, CircleCheck, CircleDot, Play, Swords } from "lucide-react";
import { Eclipse } from "lucide-react";
import { Kbd, Tooltip } from "@armada/components";
import type { TileAsk } from "@armada/protocol";
import { attachmentsOf, type HoldsReleased, type Session, type SessionsDraft } from "@armada/screens/src/draft/sessions";
import { refusalWords } from "@armada/screens/src/refusal-words";

import { useSessionsSaid } from "../../sessions-draft";
import { useFleetEvents } from "../fleet-events";
import { filled, repositoryFileOf, tilesOf, useFlightFile, useRepositoryFlight, type JobFacts, type Tile } from "./flight-controls";
import { approveTile, committedFileOf } from "./flight-controls";
import { TileAskDialog, askOfRefusal } from "./TileAskDialog";
import { glyphOf } from "./flight-glyphs";
import { HoldsReleasedBox, ReleaseHoldsDialog } from "./ReleaseHolds";
import { ReportBox } from "./ReportBox";
import { OverTheApp } from "../../session-view/OverTheApp";
import { reportBrief } from "./report-brief";
import { COMPACT, handTargets, heldOf, mute, nameOfHeldRow, offeredOf, plain, type Held, type SessionTileId } from "./session-tiles";
import { ShellBox, type ShellRun } from "./ShellBox";
import { select, setView } from "./store";
import { openRequestsInPanel } from "../../shared-panel/store";
import { toggleLayer, useLayer } from "../../session-view/layers";
import { baysLit, type SessionBays } from "../../session-view/slot-trouble";

/** The share of the context window from which the meter reads as nearly full. */
const HIGH_PERCENT = 85;

/** How long a judge that found nothing is shown as done. */
const CLEAR_MS = 2400;

/** The repository the report is about: its root and its name where it has one. */
export type ReportRepository = { root: string; name?: string };

/** `startIn` is the root a report's new Session starts in (`reportRoot`); absent, the reported Session's own. */
export function SessionTiles({ session, draft, repository, startIn, bays, onOpenLink }: { session: Session; draft: SessionsDraft; repository: ReportRepository | undefined; startIn?: string; bays?: SessionBays | undefined; onOpenLink?: (address: string) => void }) {
  const layer = useLayer(session.id);
  const flight = useRepositoryFlight(repository?.root, session.id);
  const tiles = tilesOf("job", useFlightFile(), repositoryFileOf(flight), committedFileOf(flight));
  /** The committed tile asking before its first run here, and what Fleet said to the last Run, if anything. */
  const [asking, setAsking] = useState<{ tile: Tile; ask: TileAsk; said?: string } | undefined>();
  const [pending, setPending] = useState<string | undefined>();
  const [reporting, setReporting] = useState(false);
  /** When the Judge tile was last pressed: a reading with no concerns is "No concerns" only if it landed after this. */
  const [pressedAt, setPressedAt] = useState<number | undefined>();
  const [clear, setClear] = useState(false);
  const judged = session.work?.judged;
  /** Fleet answers the press at once with the record `judging`, so the tile reads it from there; the press is working until that arrives. */
  const judging = session.work?.judging === true || pending === "judge";
  useEffect(() => {
    if (pressedAt === undefined || judged === undefined || judged.concerns !== 0 || Date.parse(judged.at) < pressedAt) return;
    setClear(true);
    const handle = window.setTimeout(() => setClear(false), CLEAR_MS);
    return () => window.clearTimeout(handle);
  }, [pressedAt, judged?.at, judged?.concerns]);
  /** What a Shell action printed, standing in place of the tiles until Close. */
  const [shell, setShell] = useState<ShellRun | undefined>();
  /** Which run the box stands for, so an answer that lands after Close or after a second press is dropped. */
  const shellAt = useRef(0);
  const closeShell = () => {
    shellAt.current += 1;
    setShell(undefined);
  };
  const [releasing, setReleasing] = useState(false);
  /** What Fleet answered, with the Session as it stood before, which names each row. */
  const [released, setReleased] = useState<{ answer: HoldsReleased; to: Session | undefined; before: Session; unticked: readonly Held[] } | undefined>();
  /** The Session a report started, kept while its message is refused so a second press tells the same one. */
  const started = useRef<string | undefined>(undefined);
  const events = useFleetEvents();
  const refused = useSessionsSaid();
  const on = session.autoLand === true;
  const { off, compact } = offeredOf(session, draft, repository?.root, (bays?.bays.length ?? 0) > 0);
  const sent = async (text: string): Promise<boolean> => (await draft.send(session.id, plain(text))) !== false;
  const run = async (tile: string, act: () => Promise<void>) => {
    if (pending !== undefined) return;
    setPending(tile);
    try {
      await act();
    } finally {
      setPending(undefined);
    }
  };
  const tileById = (id: string): Tile => tiles.find((one) => one.id === id)!;
  /** A slash command, typed in the terminal of a terminal Session and sent as a message to a hosted one. */
  const typed = (id: string, text: string, how: "terminal" | "message") => {
    if (how === "terminal") draft.command?.(session.id, text);
    else void run(id, async () => void (await sent(text)));
  };
  const press: Record<SessionTileId, () => void> = {
    ask: () => void run("ask", async () => void (await sent(tileById("ask").words ?? ""))),
    land: () =>
      void run("land", async () => {
        const land = tileById("land");
        // Fleet holds the flag and does the merging; the words tell the agent. A refusal of the flag is toasted by the draft.
        if (await draft.autoLand?.(session.id, !on)) await sent((on ? land.offWords : land.onWords) ?? "");
      }),
    compact: () => typed("compact", tileById("compact").words ?? `/${COMPACT}`, compact?.how ?? "message"),
    report: () => setReporting((open) => !open),
    map: () => setView("map"),
    judge: () =>
      void run("judge", async () => {
        if (draft.work?.judge === undefined) return;
        setClear(false);
        setPressedAt(Date.now());
        await draft.work.judge(session.id);
      }),
    release: () => setReleasing(true),
    artifacts: () => toggleLayer(session.id, "artifacts"),
    subagents: () => toggleLayer(session.id, "subagents"),
    checks: () => toggleLayer(session.id, "checks"),
    slot: () => toggleLayer(session.id, "slot"),
    requests: () => void openRequestsInPanel(session.id),
    pause: () => void run("pause", async () => void (await (session.paused === true ? draft.resume : draft.pause)?.(session.id))),
  };
  const job: JobFacts = {
    branch: attachmentsOf(session, "branch")[0]?.name,
    pr: attachmentsOf(session, "pull_request").find((one) => one.state !== "merged")?.number,
    repository: repository?.name,
  };
  const listed = (command: string): boolean => (session.terminal === true ? (draft.command === undefined ? [] : (session.commands ?? [])) : draft.commands).some((one) => one.name === command);
  const commandOf = (text: string): string => text.trim().replace(/^\//, "").split(/\s+/)[0] ?? "";
  /** Why a tile of the owner's own is off, or nothing where it is on. */
  const offCustom = (tile: Tile): string | undefined => {
    const words = tile.words ?? "";
    if (tile.kind === "page") {
      if (onOpenLink === undefined) return "Fleet does not offer this";
      const address = filled(words, job);
      return "missing" in address ? address.missing : undefined;
    }
    if (tile.kind === "shell") return (bays?.bays.length ?? 0) === 0 ? "The Job holds no bay" : undefined;
    const silent = mute(session);
    if (silent !== undefined) return silent;
    if (tile.kind === "command" && !listed(commandOf(words))) return `The agent has not listed ${commandOf(words)}`;
    return undefined;
  };
  const pressCustom = (tile: Tile) => {
    const words = tile.words ?? "";
    if (tile.kind === "words") void run(tile.id, async () => void (await sent(words)));
    else if (tile.kind === "command") typed(tile.id, words, session.terminal === true ? "terminal" : "message");
    else if (tile.kind === "page") {
      const address = filled(words, job);
      if ("url" in address) onOpenLink?.(address.url);
    } else if (tile.kind === "shell") {
      const ask = tile.committed && flight?.what === "read" ? flight.asking?.find((one) => one.id === tile.id) : undefined;
      if (tile.committed && repository !== undefined && ask?.approved !== true) {
        if (ask !== undefined) setAsking({ tile, ask });
        else runShell(tile, words);
        return;
      }
      runShell(tile, ask?.command ?? words);
    }
  };
  /** A committed tile names itself, and Fleet runs the command the repository holds for it; one not approved here comes back as an ask. */
  const runShell = (tile: Tile, line: string) => {
    const at = (shellAt.current += 1);
    const from = tile.committed && repository !== undefined ? { repository: repository.root, id: tile.id } : undefined;
    setShell({ line });
    void (from === undefined ? window.armada.runSessionShell(session.id, line) : window.armada.runSessionShell(session.id, line, from)).then((answer) => {
      if (shellAt.current !== at) return;
      const ask = answer.ok ? undefined : askOfRefusal(answer.outcome, tile.id);
      if (ask !== undefined) {
        closeShell();
        setAsking({ tile, ask });
        return;
      }
      setShell({ line, ...(answer.ok ? { ran: answer.value } : { said: refusalWords(answer.outcome) }) });
    });
  };
  const approveAndRun = async () => {
    if (asking === undefined || repository === undefined) return;
    const answer = await approveTile(repository.root, session.id, asking.ask);
    if (!answer.ok) {
      setAsking({ ...asking, said: refusalWords(answer.outcome) });
      return;
    }
    setAsking(undefined);
    runShell(asking.tile, asking.ask.command);
  };
  const release = async (to: Session | undefined, rows: readonly Held[]): Promise<boolean> => {
    const before = session;
    const unticked = heldOf(session).filter((one) => !rows.some((it) => it.kind === one.kind && it.target === one.target));
    const answer = await draft.releaseHolds?.(session.id, to?.id, rows.map(({ kind, target }) => ({ kind, target })));
    if (answer === undefined) return false;
    setReleasing(false);
    // A slot takes the branch it has checked out with it, so an unticked branch can still have moved.
    setReleased({ answer, to, before, unticked: unticked.filter((one) => !answer.moved.some((it) => it.kind === one.kind && it.target === one.target)) });
    return true;
  };
  const report = async (words: string): Promise<boolean> => {
    if (repository === undefined) return false;
    const id = started.current ?? (await draft.start(undefined, startIn ?? repository.root));
    if (id === undefined) return false;
    started.current = id;
    const brief = reportBrief(words, session, { at: new Date(), screen: "Cockpit, flight controls", root: repository.root, ...(repository.name === undefined ? {} : { repository: repository.name }), events, ...(refused === undefined ? {} : { refused }) });
    if ((await draft.send(id, plain(brief))) === false) return false;
    started.current = undefined;
    select({ kind: "session", key: `session:${id}` });
    return true;
  };
  const clashing = (bays?.clashes ?? []).map((one) => (one.one.kind === "session" && one.one.id === session.id ? one.other : one.one).label);
  const clashSaid = clashing.length === 0 ? undefined : `Write clash with ${clashing.join(", ")}`;
  const wrongs = [...(bays?.bays ?? []).flatMap((one) => (one.said === undefined ? [] : [one.said])), ...(clashSaid === undefined ? [] : [clashSaid])];
  const filledBy = session.context === undefined || session.context.window <= 0 ? undefined : Math.min(100, Math.round((session.context.tokens / session.context.window) * 100));
  const subagentsRunning = attachmentsOf(session, "subagent").filter((one) => one.state === "running").length;
  const checksRunning = session.rows.some((one) => one.kind === "check" && one.state === "running");
  const tip: Record<SessionTileId, string> = {
    ask: "Ask the Job for an update",
    land: on ? "Turn auto land off" : "Turn auto land on",
    compact: filledBy === undefined ? "Compact the Job's context" : `Context ${filledBy}% full. Compact it`,
    report: "Report an issue with the Job",
    map: "Map",
    judge: judging ? "Judge reading" : clear ? "No concerns" : "Send a judge to read the work",
    release: "Release or hand over everything the Job holds",
    artifacts: "Everything the Job made or used",
    subagents: subagentsRunning === 0 ? "Every subagent" : `Every subagent, ${subagentsRunning} running`,
    checks: checksRunning ? "The Checks the Job ran, running now" : "The Checks the Job ran",
    slot: wrongs.length === 0 ? "Every bay the Job holds" : wrongs.join(", "),
    requests: "Send a request with your cookie for its host",
    pause: session.paused === true ? "Resume" : "Pause",
  };
  const tipCustom = (tile: Tile): string => {
    const words = (tile.words ?? "").trim();
    const short = words.length > 60 ? `${words.slice(0, 60)}...` : words;
    const does = tile.kind === "words" ? `Send the Job: ${short}` : tile.kind === "command" ? `Type ${short} to the Job` : tile.kind === "shell" ? `Run ${short} in the Job's bay` : `Open ${short}`;
    return tile.hides === undefined ? does : `${does}. Hides the repository's ${tile.hides}`;
  };
  const shown = tiles.filter((one) => !one.hidden);
  // Over the whole window, never in the panel: the panel's glass is the containing block of anything fixed inside it, and clipped them.
  const dialog = releasing ? (
    <OverTheApp open>
      <ReleaseHoldsDialog session={session} held={heldOf(session)} targets={handTargets(session, draft.get())} send={release} onCancel={() => setReleasing(false)} />
    </OverTheApp>
  ) : asking !== undefined ? (
    <OverTheApp open>
      <TileAskDialog name={asking.tile.label} ask={asking.ask} {...(asking.said === undefined ? {} : { said: asking.said })} onRun={() => void approveAndRun()} onCancel={() => setAsking(undefined)} />
    </OverTheApp>
  ) : reporting ? (
    <ReportBox
      keep={`report:${session.id}`}
      send={report}
      onClose={() => {
        started.current = undefined;
        setReporting(false);
      }}
    />
  ) : null;
  if (released !== undefined) {
    return <HoldsReleasedBox released={released.answer} to={released.to} unticked={released.unticked} nameOfRow={(row) => nameOfHeldRow(released.before, row)} onClose={() => setReleased(undefined)} />;
  }
  if (shell !== undefined) return <ShellBox run={shell} onClose={closeShell} />;
  return (
    <>
      {dialog}
      <div className="deck-controls__block" data-subject="session">
        <div className="deck-controls__tiles" style={{ "--count": Math.min(shown.length, 12), "--cols": Math.min(Math.ceil(shown.length / 2), 6) } as CSSProperties}>
          {shown.map((tile) => {
            const id = tile.builtIn === undefined ? undefined : (tile.id as SessionTileId);
            const paused = id === "pause" && session.paused === true;
            const Glyph = paused ? Play : glyphOf(tile.glyph, "zap");
            const name = paused ? "Resume" : tile.label;
            const key = tile.key;
            const why = id === undefined ? offCustom(tile) : off[id];
            const reading = id === "judge" && judging;
            const cleared = id === "judge" && clear && !reading;
            const pulsing = id === "checks" && checksRunning;
            return (
              <Tooltip key={tile.id} label={why ?? (id === undefined ? tipCustom(tile) : tip[id])} {...(key === undefined ? {} : { shortcut: key })} asChild>
                <button
                  type="button"
                  className="deck-tile"
                  aria-label={name}
                  data-key={key}
                  data-act={id === "pause" ? "pause" : undefined}
                  aria-pressed={id === undefined ? undefined : id === "map" || id === "release" || id === "pause" ? false : pending === tile.id || (id === "land" && on) || layer === id}
                  data-lit={(id === "slot" && bays !== undefined && baysLit(bays)) || undefined}
                  data-on={paused || undefined}
                  data-running={pulsing || undefined}
                  aria-busy={pending === tile.id || undefined}
                  aria-disabled={why !== undefined || undefined}
                  onClick={() => why === undefined && (id === undefined ? pressCustom(tile) : press[id]())}
                >
                  {reading ? (
                    <span className="armada-session-mark" role="img" aria-label="Judge reading" data-pulsing>
                      <CircleDot size={20} strokeWidth={2} aria-hidden="true" />
                    </span>
                  ) : cleared ? (
                    <span className="armada-session-mark deck-tile__done" role="img" aria-label="No concerns">
                      <CircleCheck size={20} strokeWidth={2} aria-hidden="true" />
                    </span>
                  ) : (
                    <Glyph size={20} aria-hidden="true" />
                  )}
                  <span className="deck-tile__text">{paused ? "Resume" : tile.text}</span>
                  {id !== "land" ? null : (
                    // A mark in the corner, as Subagents carries its count, so the tile keeps its place: a filled dot while on, a ring while off.
                    <span className="deck-tile__state" role="img" aria-label={on ? "Auto land on" : "Auto land off"} data-on={on || undefined}>
                      {on ? <CircleDot size={12} strokeWidth={2} aria-hidden="true" /> : <Circle size={12} strokeWidth={2} aria-hidden="true" />}
                    </span>
                  )}
                  {id !== "slot" || clashSaid === undefined ? null : (
                    // A write clash is marked here and not in the Session's header: the owner's ask of 17 Oct 2026.
                    <span className="deck-tile__clash" role="img" aria-label={clashSaid}>
                      <Swords size={12} strokeWidth={2} aria-hidden="true" />
                    </span>
                  )}
                  {tile.hides === undefined ? null : (
                    <span className="deck-tile__hides" role="img" aria-label={`Hides the repository's ${tile.hides}`}>
                      <Eclipse size={12} strokeWidth={2.5} aria-hidden="true" />
                    </span>
                  )}
                  {id !== "subagents" || subagentsRunning === 0 ? null : (
                    // The number running: the owner asked for it by name, the one place a figure is drawn beside what it counts.
                    <span className="deck-tile__count" role="img" aria-label={`${subagentsRunning} subagents running`}>
                      {subagentsRunning}
                    </span>
                  )}
                  {id !== "compact" || filledBy === undefined ? null : (
                    <span className="deck-tile__meter" role="meter" aria-label="Context used" aria-valuemin={0} aria-valuemax={100} aria-valuenow={filledBy} data-high={filledBy >= HIGH_PERCENT || undefined}>
                      <span style={{ width: `${filledBy}%` }} />
                    </span>
                  )}
                  {key === undefined ? null : (
                    <Kbd className="deck-tile__key" aria-hidden="true">
                      {key}
                    </Kbd>
                  )}
                </button>
              </Tooltip>
            );
          })}
        </div>
      </div>
    </>
  );
}
