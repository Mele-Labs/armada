// A watch in the side panel: its readings over time with what the agent made of each, the parts of its last reading with those set aside
// struck through beside the sentence that set them aside, the whole command Fleet runs for the Job, and the last output of a broken reading.
// Its link out is on the heading's line, and Stop ends it while it is live. Read from the Session, as `watches.changed` last carried it.
// A failed part shows its own link, the exact lines it failed on and the end of its log; a weighing shows the evidence it carried; and a
// reading that kept a log opens it in the Log tab, read from Fleet, with one copy control.

import { useEffect, useState } from "react";
import { Button, CopyAct, Stamp } from "@armada/components";
import type { WatchPart, WatchReading, WatchSummary, WatchVerdict } from "@armada/protocol";

import { openLink } from "../commands";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { readingSays, readingState, readingsOf, verdictSaid, verdictsOn, watchSaid, WatchMark } from "../session-view/watch-marks";
import { Open, useBack } from "./panels";
import { SidePanel } from "./side";
import { closePanel } from "./store";
import type { PanelItem } from "./store";

type Item = Extract<PanelItem, { kind: "watch" }>;

/** `60s`, `2m`, `2h`: a cadence or a limit as a recipe writes one. */
const span = (seconds: number): string => (seconds % 3600 === 0 ? `${seconds / 3600}h` : seconds % 60 === 0 ? `${seconds / 60}m` : `${seconds}s`);

/** The parts of the newest reading that named any. */
const partsOf = (watch: WatchSummary): WatchPart[] => [...readingsOf(watch)].reverse().find((one): one is Extract<WatchReading, { kind: "read" }> => one.kind === "read" && one.parts !== undefined)?.parts ?? [];

/** The newest broken reading that kept its output. */
const brokenOf = (watch: WatchSummary) => [...readingsOf(watch)].reverse().find((one): one is Extract<WatchReading, { kind: "broken" }> => one.kind === "broken" && one.output !== undefined);

/** The link the newest reading gave. */
const linkOf = (watch: WatchSummary): string | undefined => [...readingsOf(watch)].reverse().find((one): one is Extract<WatchReading, { kind: "read" }> => one.kind === "read" && one.link !== undefined)?.link;

/** A reading's times: its own, or a run's first above its last. */
function When({ reading }: { reading: WatchReading }) {
  if (reading.last_at === undefined) return <Stamp at={reading.at} className="side-watch__at" />;
  return (
    <span className="side-watch__at side-watch__span">
      <Stamp at={reading.at} />
      <Stamp at={reading.last_at} />
    </span>
  );
}

/** What a weighing carried to back it: each link, log and exact error, one a line. */
function Evidence({ verdict }: { verdict: WatchVerdict }) {
  const { links = [], logs = [], errors = [] } = verdict.evidence ?? {};
  if (links.length + logs.length + errors.length === 0) return null;
  return (
    <ul className="side-watch__evidence" aria-label="Evidence">
      {errors.map((one) => (
        <li key={`e:${one}`}>
          <pre className="side-watch__error mono">{one}</pre>
        </li>
      ))}
      {links.map((one) => (
        <li key={`l:${one}`} className="mono">
          {one}
        </li>
      ))}
      {logs.map((one) => (
        <li key={`g:${one}`} className="mono">
          {one}
        </li>
      ))}
    </ul>
  );
}

/** Fleet sends back-to-back readings that read the same as one run, drawn as one row with how many it stands for. */
function Readings({ watch, onLog }: { watch: WatchSummary; onLog: (log: string) => void }) {
  return (
    <ol className="side-watch__readings" aria-label="Readings">
      {[...readingsOf(watch)].reverse().map((reading) => (
        <li key={reading.at} className="side-watch__reading">
          <When reading={reading} />
          <WatchMark state={readingState(reading)} />
          <span className="side-watch__says">{readingSays(reading)}</span>
          {(reading.count ?? 1) > 1 ? (
            <span className="side-watch__count" aria-label={`Read ${String(reading.count)} times in a row`}>
              ×{reading.count}
            </span>
          ) : reading.log !== undefined ? (
            <Button variant="ghost" size="sm" aria-label={`Log of ${readingSays(reading)}`} onClick={() => onLog(reading.log!)}>
              Log
            </Button>
          ) : (
            <span />
          )}
          {verdictsOn(watch, reading).map((one) => (
            <div key={`${one.at}:${one.verdict}`} className="side-watch__verdict" data-verdict={one.verdict}>
              <p className="side-watch__says">{verdictSaid(one)}</p>
              <Evidence verdict={one} />
            </div>
          ))}
        </li>
      ))}
    </ol>
  );
}

/** What a failed part gave of itself: where to look, the exact lines it failed on, and the end of its log. */
function PartEvidence({ part }: { part: WatchPart }) {
  if (part.link === undefined && part.error === undefined && part.tail === undefined) return null;
  return (
    <div className="side-watch__part-evidence" role="group" aria-label={`${part.name} evidence`}>
      {part.link === undefined ? null : (
        <span className="side-watch__part-link">
          <span className="mono">{part.link}</span>
          <Open address={part.link} onOpenLink={(address) => void openLink(address)} />
        </span>
      )}
      {part.error === undefined ? null : (
        <pre className="side-watch__error mono" aria-label={`${part.name} failed on`}>
          {part.error}
        </pre>
      )}
      {part.tail === undefined ? null : (
        <details className="side-watch__tail">
          <summary>The end of its log</summary>
          <pre className="side-log__lines mono">{part.tail}</pre>
        </details>
      )}
    </div>
  );
}

function Parts({ watch }: { watch: WatchSummary }) {
  const aside = new Map((watch.set_aside ?? []).map((one) => [one.part, one.sentence]));
  const parts = partsOf(watch);
  // A part set aside that the newest reading no longer names still shows why it was set aside.
  const gone = [...aside.keys()].filter((name) => !parts.some((one) => one.name === name));
  return (
    <ul className="side-watch__parts" aria-label="Parts">
      {parts.map((part) => {
        const sentence = aside.get(part.name);
        return (
          <li key={part.name} className="side-watch__part" data-aside={sentence === undefined ? undefined : ""}>
            <WatchMark state={part.state} />
            {sentence === undefined ? <span className="side-watch__part-name mono">{part.name}</span> : <s className="side-watch__part-name mono">{part.name}</s>}
            {sentence === undefined ? null : <span className="side-watch__aside">Not ours: {sentence}</span>}
            <PartEvidence part={part} />
          </li>
        );
      })}
      {gone.map((name) => (
        <li key={name} className="side-watch__part" data-aside="">
          <span />
          <s className="side-watch__part-name mono">{name}</s>
          <span className="side-watch__aside">Not ours: {aside.get(name)}</span>
        </li>
      ))}
    </ul>
  );
}

function Command({ watch }: { watch: WatchSummary }) {
  return (
    <section className="side-watch__command" aria-label="Command">
      <pre className="side-log__lines mono">{watch.command.join(" ")}</pre>
      <dl className="side-facts">
        {watch.recipe === undefined ? null : (
          <div className="side-facts__row">
            <dt>Recipe</dt>
            <dd className="mono">{watch.recipe}</dd>
          </div>
        )}
        <div className="side-facts__row">
          <dt>Every</dt>
          <dd className="mono">{span(watch.every_seconds)}</dd>
        </div>
        <div className="side-facts__row">
          <dt>Gives up after</dt>
          <dd className="mono">{span(watch.gives_up_after_seconds)}</dd>
        </div>
        <div className="side-facts__row">
          <dt>Started</dt>
          <dd>
            <Stamp at={watch.started_at} />
          </dd>
        </div>
      </dl>
    </section>
  );
}

/** A reading's whole log, read from Fleet: its file, one copy control, and its text. */
function Log({ watch, log }: { watch: WatchSummary; log: string }) {
  const draft = useSessionsDraft();
  const [read, setRead] = useState<{ log: string; text?: string; said?: string }>();
  const reader = draft?.readWatchLog;
  useEffect(() => {
    let live = true;
    if (reader === undefined) return;
    void reader(watch.id, log).then((answer) => {
      if (live) setRead(answer.ok ? { log, text: answer.text } : { log, said: answer.said });
    });
    return () => {
      live = false;
    };
  }, [reader, watch.id, log]);
  const shown = read?.log === log ? read : undefined;
  return (
    <section className="side-log" aria-label="Log">
      <div className="side-watch__log-head">
        <span className="mono side-watch__log-path">{log}</span>
        {shown?.text === undefined ? null : <CopyAct text={shown.text} label="Copy log" />}
      </div>
      {shown === undefined ? <p className="side-watch__says">Reading the log</p> : shown.text === undefined ? <p className="side-watch__says">{shown.said}</p> : <pre className="side-log__lines mono side-watch__log">{shown.text}</pre>}
    </section>
  );
}

export function WatchPanel({ item }: { item: Item }) {
  const sessions = useSessions();
  const draft = useSessionsDraft();
  const back = useBack();
  const [tab, setTab] = useState("readings");
  const [picked, setPicked] = useState<string>();
  const watch = sessions.find((one) => one.id === item.sessionId)?.watches?.find((one) => one.id === item.id);
  if (watch === undefined) return null;
  const broken = brokenOf(watch);
  const link = linkOf(watch);
  const logs = [...readingsOf(watch)].reverse().flatMap((one) => (one.log === undefined ? [] : [one.log]));
  const log = picked !== undefined && logs.includes(picked) ? picked : logs[0];
  const tabs = [
    { id: "readings", label: "Readings" },
    { id: "parts", label: "Parts" },
    { id: "command", label: "Command" },
    ...(broken === undefined ? [] : [{ id: "output", label: "Output" }]),
    ...(log === undefined ? [] : [{ id: "log", label: "Log" }]),
  ];
  const stop = draft?.stopWatch;
  return (
    <SidePanel
      title={`Watch ${watch.name}`}
      heading={watch.name}
      beside={link === undefined ? undefined : <Open address={link} onOpenLink={(address) => void openLink(address)} />}
      meta={
        <>
          <WatchMark state={watch.state} said={watchSaid(watch)} />
          {watch.follows === undefined ? null : <span className="mono">#{watch.follows}</span>}
        </>
      }
      {...(watch.live && stop !== undefined
        ? {
            acts: (
              <Button variant="secondary" size="sm" onClick={() => stop(item.sessionId, watch.id)}>
                Stop
              </Button>
            ),
          }
        : {})}
      tabs={tabs}
      tab={tabs.some((one) => one.id === tab) ? tab : "readings"}
      onTab={setTab}
      back={back}
      onClose={closePanel}
    >
      {tab === "readings" ? <Readings watch={watch} onLog={(one) => (setPicked(one), setTab("log"))} /> : null}
      {tab === "parts" ? <Parts watch={watch} /> : null}
      {tab === "command" ? <Command watch={watch} /> : null}
      {tab === "log" && log !== undefined ? <Log watch={watch} log={log} /> : null}
      {tab === "output" && broken !== undefined ? (
        <section className="side-log" aria-label="Output">
          <p className="side-watch__says">{broken.why}</p>
          <pre className="side-log__lines mono">{broken.output}</pre>
        </section>
      ) : null}
    </SidePanel>
  );
}
