// Operator (mock, on fixtures): throw an idea, a feature or a bug at the Operator from the phone, and read what it did with
// the last sends. The words, the kind chips and the repository follow native's composer (`operator/composer.rs`): a kind is a
// hint and none is none, and pressing the picked chip again unpicks it. A Mac asleep takes a send into a row that waits for it.
// Nothing here calls a Mac: `fixture/operator.ts` answers, and the Operator's decision there is a stand-in.

import { useEffect, useMemo, useRef, useState } from "react";
import type { LucideIcon } from "lucide-react";
import {
  Bookmark,
  Bug,
  CircleHelp,
  CircleDot,
  CornerDownRight,
  Lightbulb,
  Moon,
  Send as SendGlyph,
  Sparkles,
  TriangleAlert,
} from "lucide-react";
import { Badge, Button, Select, Stamp, Textarea, Toast } from "@armada/components";

import {
  decide,
  operatorMacs,
  operatorSends,
  type Kind,
  type OperatorMac,
  type Send,
  type Stage,
  type Way,
} from "./fixture/operator";
import { go, jobPath } from "./router";
import { Band } from "./ui";

/** What the words are kept under, as Dispatch keeps its own (`pocket:dispatch`). */
const KEEP = "pocket:operator";
const WORDS = "operator-words";
/** How long the Operator reads a send before the mock decides. */
const READING_MS = 900;
const TOAST_MS = 6000;

const KINDS: {
  kind: Kind;
  word: string;
  icon: LucideIcon;
  hue: string;
}[] = [
  {
    kind: "idea",
    word: "Idea",
    icon: Lightbulb,
    hue: "awaiting-review",
  },
  {
    kind: "feature",
    word: "Feature",
    icon: Sparkles,
    hue: "handed",
  },
  {
    kind: "bug",
    word: "Bug",
    icon: Bug,
    hue: "completed-failed",
  },
];

const WAYS: Record<
  Way,
  {
    word: string;
    icon: LucideIcon;
    hue: string;
  }
> = {
  started: {
    word: "Started",
    icon: CircleDot,
    hue: "running",
  },
  handed: {
    word: "Handed",
    icon: CornerDownRight,
    hue: "handed",
  },
  asked: {
    word: "Asked",
    icon: CircleHelp,
    hue: "awaiting-review",
  },
  parked: {
    word: "Parked",
    icon: Bookmark,
    hue: "not-started",
  },
};

type Face = {
  word: string;
  icon: LucideIcon;
  hue: string;
};

const faceOf = (stage: Stage): Face => {
  switch (stage.phase) {
    case "reading":
      return {
        word: "Sent",
        icon: SendGlyph,
        hue: "not-started",
      };
    case "dispatched":
      return WAYS[stage.way];
    case "queued":
      return {
        word: "Waiting",
        icon: Moon,
        hue: "escalated",
      };
    case "refused":
      return {
        word: "Refused",
        icon: TriangleAlert,
        hue: "completed-failed",
      };
  }
};

type Shown = {
  text: string;
  hue: string;
  to?: string;
};

/** One send, as the Operator's list draws it. */
function SendRow({
  send,
  macs,
  onTakeBack,
}: {
  send: Send;
  macs: OperatorMac[];
  onTakeBack: (send: Send) => void;
}) {
  const { stage } = send;
  const face = faceOf(stage);
  const mac = macs.find((one) => one.name === send.mac);
  const job =
    stage.phase === "dispatched" && stage.session_id !== undefined && mac !== undefined
      ? jobPath(mac.origin, stage.session_id)
      : undefined;
  const title = stage.phase === "dispatched" ? stage.title : undefined;
  const needs = stage.phase === "queued" || stage.phase === "refused" || (stage.phase === "dispatched" && stage.way === "asked");
  return (
    <li className="deck-list__item">
      <div className="pk-send" data-hue={face.hue} data-needs={needs || undefined}>
        <div className="pk-send__head">
          <Badge status={face.hue} icon={face.icon}>
            {face.word}
          </Badge>
          {title !== undefined && job !== undefined ? (
            <button type="button" className="pk-send__title" onClick={() => go(job)}>
              {title}
            </button>
          ) : (
            <span className="pk-send__title">{title ?? send.words}</span>
          )}
          <Stamp first="age" at={send.at} />
        </div>
        {title === undefined ? null : <p className="pk-send__words">{send.words}</p>}
        <p className="pk-send__meta">
          <span>{send.repository}</span>
          <span className="pk-row__mac">{send.mac}</span>
        </p>
        {stage.phase === "queued" ? (
          <div className="pk-send__why">
            <p>
              {send.mac} fell asleep <Stamp first="age" at={stage.since * 1000} />. This sends when{" "}
              {send.mac} wakes.
            </p>
            <div className="pk-send__acts">
              <Button variant="secondary" size="sm" onClick={() => onTakeBack(send)}>
                Take back
              </Button>
            </div>
          </div>
        ) : null}
        {stage.phase === "refused" ? (
          <div className="pk-send__why">
            <p>{stage.why}</p>
            <div className="pk-send__acts">
              <Button variant="secondary" size="sm" onClick={() => onTakeBack(send)}>
                Take back
              </Button>
            </div>
          </div>
        ) : null}
        {stage.phase === "dispatched" && stage.way === "asked" ? (
          <p className="pk-send__why">The Operator asked about this. Its question is on Bridge.</p>
        ) : null}
      </div>
    </li>
  );
}

/** A choice in the composer: a toggle with its hue as fill when it stands, or a radio of a group. */
function Chip({
  hue,
  icon: Icon,
  checked,
  role,
  onPress,
  children,
}: {
  hue: string;
  icon?: LucideIcon;
  checked: boolean;
  role?: "radio";
  onPress: () => void;
  children: string;
}) {
  return (
    <button
      type="button"
      className="pk-chip"
      data-hue={hue}
      {...(role === "radio"
        ? {
            role,
            "aria-checked": checked,
          }
        : {
            "aria-pressed": checked,
          })}
      onClick={onPress}
    >
      {Icon === undefined ? null : <Icon size={14} strokeWidth={2} aria-hidden="true" />}
      {children}
    </button>
  );
}

export function Operator({ asleep }: { asleep: boolean }) {
  const [now] = useState(() => Date.now());
  const macs = useMemo(
    () => operatorMacs(window.location.origin, asleep, now),
    [
      asleep,
      now,
    ],
  );
  const [sends, setSends] = useState<Send[]>(() => operatorSends(asleep, now));
  const [line, setLine] = useState("");
  const [kind, setKind] = useState<Kind | undefined>(undefined);
  const [repository, setRepository] = useState("armada");
  const [picked, setPicked] = useState("studio");
  const [toast, setToast] = useState<Shown | undefined>(undefined);
  const made = useRef(0);
  const timers = useRef<number[]>([]);
  useEffect(
    () => () => {
      timers.current.forEach((one) => window.clearTimeout(one));
    },
    [],
  );
  const later = (act: () => void, ms: number) => timers.current.push(window.setTimeout(act, ms));

  const repositories = [...new Set(macs.flatMap((mac) => mac.repositories))];
  /** A repository checked out on one Mac only is offered on that Mac only. */
  const offered = macs.filter((mac) => mac.repositories.includes(repository));
  const mac = offered.find((one) => one.name === picked) ?? offered[0];

  const tell = (stage: Stage, mac: OperatorMac | undefined) => {
    if (stage.phase !== "dispatched") return;
    const to =
      stage.session_id !== undefined && mac !== undefined
        ? jobPath(mac.origin, stage.session_id)
        : undefined;
    const text = (
      {
        started: `Started ${stage.title ?? ""}`,
        handed: `Handed to ${stage.title ?? ""}`,
        asked: "Asked a question",
        parked: `Parked ${stage.title ?? ""}`,
      } satisfies Record<Way, string>
    )[stage.way];
    setToast({
      text,
      hue: WAYS[stage.way].hue,
      ...(to === undefined
        ? {}
        : {
            to,
          }),
    });
    later(() => setToast(undefined), TOAST_MS);
  };

  const send = () => {
    if (mac === undefined || line.trim() === "") return;
    made.current += 1;
    const one: Send = {
      id: `n${made.current}`,
      at: new Date().toISOString(),
      mac: mac.name,
      repository,
      words: line.trim(),
      ...(kind === undefined
        ? {}
        : {
            kind,
          }),
      stage:
        mac.link.state === "asleep"
          ? {
              phase: "queued",
              since: mac.link.since,
            }
          : {
              phase: "reading",
            },
    };
    setSends((was) => [
      one,
      ...was,
    ]);
    setLine("");
    setKind(undefined);
    if (mac.link.state === "asleep") return;
    later(() => {
      const stage = decide(one);
      setSends((was) => was.map((row) => (row.id === one.id ? { ...row, stage } : row)));
      tell(stage, mac);
    }, READING_MS);
  };

  const takeBack = (back: Send) => {
    setSends((was) => was.filter((row) => row.id !== back.id));
    setLine((was) => (was.trim() === "" ? back.words : `${was.trimEnd()}\n\n${back.words}`));
    setKind(back.kind);
    setRepository(back.repository);
    setPicked(back.mac);
    window.requestAnimationFrame(() => document.getElementById(WORDS)?.focus());
  };

  return (
    <section className="pk-screen" aria-label="Operator">
      <Band title="Operator" onBack={() => go("/")} back="Home" />
      <div className="pk-body pk-home">
        <ul className="deck-list__rows" aria-label="Sends">
          {sends.map((one) => (
            <SendRow key={one.id} send={one} macs={macs} onTakeBack={takeBack} />
          ))}
        </ul>
      </div>
      <div className="pk-toasts">
        {toast === undefined ? null : (
          <Toast
            status={toast.hue}
            {...(toast.to === undefined
              ? {}
              : {
                  onPress: () => go(toast.to!),
                })}
          >
            {toast.text}
          </Toast>
        )}
      </div>
      <footer className="pk-bar pk-composer" aria-label="Composer">
        <Textarea
          id={WORDS}
          keep={KEEP}
          rows={2}
          aria-label="Message"
          value={line}
          autoCapitalize="sentences"
          onChange={(event) => setLine(event.target.value)}
        />
        <div className="pk-composer__row">
          <div className="pk-chips" role="group" aria-label="Kind">
            {KINDS.map((one) => (
              <Chip
                key={one.kind}
                hue={one.hue}
                icon={one.icon}
                checked={kind === one.kind}
                onPress={() => setKind(kind === one.kind ? undefined : one.kind)}
              >
                {one.word}
              </Chip>
            ))}
          </div>
          <Button
            variant="primary"
            disabled={line.trim() === "" || mac === undefined}
            onClick={send}
          >
            Send
          </Button>
        </div>
        <div className="pk-composer__row">
          <div className="pk-composer__repository">
            <Select
              aria-label="Repository"
              value={repository}
              onChange={(event) => setRepository(event.target.value)}
            >
              {repositories.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </Select>
          </div>
          <div className="pk-chips" role="radiogroup" aria-label="Mac">
            {offered.map((one) => (
              <Chip
                key={one.name}
                role="radio"
                hue={one.link.state === "asleep" ? "escalated" : "completed-success"}
                icon={one.link.state === "asleep" ? Moon : undefined}
                checked={mac === one}
                onPress={() => setPicked(one.name)}
              >
                {one.link.state === "asleep" ? `${one.name}, asleep` : one.name}
              </Chip>
            ))}
          </div>
        </div>
        {mac !== undefined && mac.link.state === "asleep" ? (
          <p className="pk-fact pk-composer__fact" role="status">
            {mac.name} fell asleep <Stamp first="age" at={mac.link.since * 1000} />. A send waits in
            the list until it wakes.
          </p>
        ) : null}
      </footer>
    </section>
  );
}
