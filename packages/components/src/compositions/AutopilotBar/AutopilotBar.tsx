// One repository's Autopilot, on every Cockpit filter so Pause is one press away: the switch beside its
// name, what it is doing, its caps used of their limit, and Pause, Resume and Stop. Off, the switch opens
// the start form, and nothing starts until the person has seen what Fleet read and pressed Start. #2092,
// `docs/concepts/autopilot.md`.

import { useEffect, useRef, useState, type CSSProperties, type FormEvent } from "react";
import { Plane } from "lucide-react";

import { Button } from "../../primitives/Button/Button";
import { Input } from "../../primitives/Input/Input";
import { Switch } from "../../primitives/Switch/Switch";
import { Textarea } from "../../primitives/Textarea/Textarea";

/** One cap, used of its limit. `fill` is 0 to 1; `reached` lights it. */
export type AutopilotCap = { label: string; value: string; fill: number; reached: boolean };

/** The four settings as the start form holds them, each a field's text. */
export type AutopilotFields = { atOnce: string; until: string; spend: string; jobs: string };

/** What Fleet read the sentence as: the fields, and for each whether the sentence said it at all. */
export type AutopilotReading = { fields: AutopilotFields; said: Readonly<Record<keyof AutopilotFields, boolean>> };

export type AutopilotPress = "pause" | "resume" | "stop" | "read" | "start";

export type AutopilotBarProps = {
  /** The repository it works, named where several bars are drawn: on All. */
  repository?: string | undefined;
  /** `off` where there is none, or the last one ended. */
  state: "off" | "running" | "paused";
  /** The person's sentence, while it runs. */
  instruction?: string | undefined;
  /** How the last one ended, where a cap ended it: the bar says which. */
  ended?: string | undefined;
  /** Why its last pass picked nothing, in Fleet's words. */
  fault?: string | undefined;
  caps: readonly AutopilotCap[];
  /** What Stop does to this run's picks, named. */
  stopping: string;
  /** The press Fleet has not answered. */
  pending?: AutopilotPress | undefined;
  /** What Fleet refused, said where the press was. */
  refused?: string | undefined;
  onPause: () => void;
  onResume: () => void;
  onStop: () => void;
  /** The start form is open: the switch was turned on, or the one bar sent a sentence here. */
  starting: boolean;
  onStarting: (open: boolean) => void;
  sentence: string;
  onSentence: (sentence: string) => void;
  /** What Fleet read the sentence as. Absent until it is read. */
  reading?: AutopilotReading | undefined;
  onRead: () => void;
  onStart: (fields: AutopilotFields) => void;
};

const STATE: Record<AutopilotBarProps["state"], string> = { off: "Off", running: "Running", paused: "Paused · started Jobs finish" };

const FIELDS: readonly (readonly [keyof AutopilotFields, string, string])[] = [
  ["atOnce", "At once", "1"],
  ["until", "Until", "no time set"],
  ["spend", "Spend, in dollars", "no cap"],
  ["jobs", "Jobs", "no cap"],
];

function Caps({ caps }: { caps: readonly AutopilotCap[] }) {
  return (
    <dl className="armada-autopilot__caps">
      {caps.map((cap) => (
        <div key={cap.label} className="armada-autopilot__cap" data-reached={cap.reached || undefined}>
          <dt>{cap.label}</dt>
          <dd>
            {cap.value}
            {cap.reached ? <span className="armada-autopilot__reached"> · reached</span> : null}
          </dd>
          <dd className="armada-autopilot__meter" aria-hidden="true" style={{ ["--fill" as string]: `${Math.round(Math.min(1, Math.max(0, cap.fill)) * 100)}%` } as CSSProperties} />
        </div>
      ))}
    </dl>
  );
}

function StartForm({ sentence, onSentence, reading, onRead, onStart, pending }: Pick<AutopilotBarProps, "sentence" | "onSentence" | "reading" | "onRead" | "onStart" | "pending">) {
  const [fields, setFields] = useState<AutopilotFields | undefined>(reading?.fields);
  // A new reading replaces whatever was typed over the last one.
  useEffect(() => setFields(reading?.fields), [reading]);
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (fields === undefined) return onRead();
    onStart(fields);
  };
  return (
    <form className="armada-autopilot__start" aria-label="Start Autopilot" onSubmit={submit}>
      <Textarea
        label="Tell Autopilot what to work on, in your own words"
        rows={2}
        value={sentence}
        onChange={(event) => onSentence(event.currentTarget.value)}
      />
      {fields === undefined || reading === undefined ? (
        <span className="armada-autopilot__foot">
          <span>Fleet reads it as settings for you to check. Nothing starts until you press Start.</span>
          <Button variant="secondary" size="sm" type="submit" disabled={sentence.trim() === ""} pending={pending === "read"}>
            {pending === "read" ? "Reading…" : "Read it"}
          </Button>
        </span>
      ) : (
        <>
          <span className="armada-autopilot__read">
            Fleet read this as:
            <Button variant="ghost" size="sm" onClick={onRead} disabled={sentence.trim() === "" || pending !== undefined} pending={pending === "read"}>
              Read again
            </Button>
          </span>
          <span className="armada-autopilot__fields">
            {FIELDS.map(([key, label, placeholder]) => (
              <span key={key} className="armada-autopilot__field">
                <Input
                  label={label}
                  mono
                  value={fields[key]}
                  placeholder={placeholder}
                  onChange={(event) => {
                    const value = event.currentTarget.value;
                    setFields((was) => (was === undefined ? was : { ...was, [key]: value }));
                  }}
                />
                {reading.said[key] ? null : <span className="armada-autopilot__said">not said</span>}
              </span>
            ))}
          </span>
          <span className="armada-autopilot__foot">
            <span>Clear picks start on their own. Picks it is unsure of are set aside for you. A cap stops the next pick, never a Job that is running.</span>
            <Button variant="primary" size="sm" type="submit" disabled={pending !== undefined && pending !== "start"} pending={pending === "start"}>
              Start Autopilot
            </Button>
          </span>
        </>
      )}
    </form>
  );
}

export function AutopilotBar(props: AutopilotBarProps) {
  const { repository, state, instruction, ended, fault, caps, stopping, pending, refused, onPause, onResume, onStop, starting, onStarting } = props;
  const [confirming, setConfirming] = useState(false);
  const on = state !== "off";
  // Stop answered: the question it asked has nothing left to ask.
  useEffect(() => {
    if (!on) setConfirming(false);
  }, [on]);
  const keep = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (confirming) keep.current?.focus();
  }, [confirming]);
  const flip = () => (on ? setConfirming(true) : onStarting(!starting));
  return (
    <section className="armada-autopilot" aria-label="Autopilot" data-state={state}>
      <div className="armada-autopilot__head">
        <span className="armada-autopilot__name">
          <Switch checked={on || starting} onChange={flip}>
            <span className="armada-autopilot__label">
              <Plane size={16} aria-hidden="true" />
              Autopilot
              {repository === undefined ? null : <span className="armada-autopilot__repository">{repository}</span>}
            </span>
          </Switch>
        </span>
        <span className="armada-autopilot__state" role="status">
          {STATE[state]}
        </span>
        {on && instruction !== undefined ? <span className="armada-autopilot__instruction">“{instruction}”</span> : null}
        {!on && ended !== undefined ? <span className="armada-autopilot__instruction">{ended}</span> : null}
        {on ? (
          <span className="armada-autopilot__acts">
            {state === "paused" ? (
              <Button variant="secondary" size="sm" onClick={onResume} disabled={pending !== undefined && pending !== "resume"} pending={pending === "resume"}>
                Resume
              </Button>
            ) : (
              <Button variant="secondary" size="sm" onClick={onPause} disabled={pending !== undefined && pending !== "pause"} pending={pending === "pause"}>
                Pause
              </Button>
            )}
            <Button variant="destructive" size="sm" onClick={() => setConfirming(true)} disabled={pending !== undefined || confirming}>
              Stop
            </Button>
          </span>
        ) : null}
      </div>
      {on && confirming ? (
        <div className="armada-autopilot__confirm" role="group" aria-label="Stop Autopilot">
          <span>{stopping}</span>
          <Button ref={keep} variant="secondary" size="sm" onClick={() => setConfirming(false)}>
            {state === "paused" ? "Keep it paused" : "Keep running"}
          </Button>
          <Button variant="destructive" size="sm" onClick={onStop} pending={pending === "stop"}>
            Stop Autopilot
          </Button>
        </div>
      ) : null}
      {refused === undefined ? null : (
        <p className="armada-autopilot__refused" role="alert">
          {refused}
        </p>
      )}
      {on && fault !== undefined ? <p className="armada-autopilot__fault">Its last pass picked nothing. {fault}</p> : null}
      {on || ended !== undefined ? <Caps caps={caps} /> : null}
      {!on && starting ? <StartForm {...props} /> : null}
    </section>
  );
}
