// Operator: throw an idea, a feature or a bug at the Operator from the phone, and read what it did with the last
// sends. The words, the kind chips and the repository follow native's composer (`operator/composer.rs`): a kind is a
// hint and none is none, and pressing the picked chip again unpicks it. A send goes to the Mac picked; one for a Mac that
// does not answer goes through another and waits in the list, with Take back.
// The rows and what they do are `sends.ts`.

import { useEffect, useRef, useState } from "react";
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

import { getJson } from "./client";
import { start, useFleet } from "./fleet";
import type { RepositoriesBody } from "./gateway";
import {
  asleep,
  dispatch,
  receiverOf,
  takeBack,
  useSends,
  watchOperator,
  nameOf,
  type Kind,
  type Send,
  type Stage,
} from "./sends";
import { go, jobPath } from "./router";
import { Band } from "./ui";

type Way = "started" | "handed" | "asked" | "parked";

/** What the words are kept under, as Dispatch keeps its own (`pocket:dispatch`). */
const KEEP = "pocket:operator";
const WORDS = "operator-words";
/** The repositories each Mac last listed, so one that is asleep still offers its own. */
const LISTED = "pocket:operator:repositories";
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
function SendRow({ send, onTakeBack }: { send: Send; onTakeBack: (send: Send) => void }) {
  const { stage } = send;
  const face = faceOf(stage);
  const job =
    stage.phase === "dispatched" && stage.session_id !== undefined && send.origin !== undefined
      ? jobPath(send.origin, stage.session_id)
      : undefined;
  const title = stage.phase === "dispatched" ? stage.title : undefined;
  const needs = stage.phase === "refused" || stage.phase === "queued" || (stage.phase === "dispatched" && stage.way === "asked");
  const mac = send.mac === "" ? "That Mac" : send.mac;
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
          {send.repository === undefined ? null : <span>{send.repository}</span>}
          {send.mac === "" ? null : <span className="pk-row__mac">{send.mac}</span>}
        </p>
        {stage.phase === "queued" ? (
          <div className="pk-send__why">
            <p>
              {mac} did not answer. This sends when {mac} wakes.
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

const kept = (): Record<string, string[]> => {
  try {
    return JSON.parse(window.localStorage.getItem(LISTED) ?? "{}") as Record<string, string[]>;
  } catch {
    return {};
  }
};

export function Operator() {
  const fleet = useFleet();
  const sends = useSends();
  const macs = fleet.macs.filter((mac) => mac.paired);
  const [listed, setListed] = useState(kept);
  const [line, setLine] = useState("");
  const [kind, setKind] = useState<Kind | undefined>(undefined);
  const [repository, setRepository] = useState("");
  const [picked, setPicked] = useState("");
  const [trouble, setTrouble] = useState("");
  const [toast, setToast] = useState<Shown | undefined>(undefined);
  const told = useRef(new Set<string>());
  const timers = useRef<number[]>([]);
  useEffect(
    () => () => {
      timers.current.forEach((one) => window.clearTimeout(one));
    },
    [],
  );
  const later = (act: () => void, ms: number) => timers.current.push(window.setTimeout(act, ms));
  const origins = macs.map((mac) => mac.machine.origin).join(" ");
  useEffect(() => void start(), []);
  useEffect(() => watchOperator(fleet.macs), [origins]);
  useEffect(() => {
    let gone = false;
    macs
      .filter((mac) => !asleep(mac))
      .forEach((mac) =>
        getJson<RepositoriesBody>(mac.machine, "/api/repositories").then(
          (names) => {
            if (gone) return;
            const all = {
              ...kept(),
              [mac.machine.origin]: names,
            };
            try {
              window.localStorage.setItem(LISTED, JSON.stringify(all));
            } catch {
              // A Mac asleep at the next start offers nothing.
            }
            setListed(all);
          },
          () => undefined,
        ),
      );
    return () => {
      gone = true;
    };
  }, [
    origins,
    macs.map(asleep).join(" "),
  ]);

  const repositories = [...new Set(macs.flatMap((mac) => listed[mac.machine.origin] ?? []))];
  const named = repositories.includes(repository) ? repository : (repositories[0] ?? "");
  /** A repository checked out on one Mac only is offered on that Mac only. */
  const offered = macs.filter((mac) => (listed[mac.machine.origin] ?? []).includes(named));
  const mac = offered.find((one) => one.machine.origin === picked) ?? offered[0];
  /** The chosen Mac takes its own send; one that is asleep is sent for through another Mac, which keeps it. */
  const receiver = mac !== undefined && !asleep(mac) ? mac : receiverOf(fleet.macs);

  const tell = (stage: Stage, origin: string | undefined) => {
    if (stage.phase !== "dispatched") return;
    const to =
      stage.session_id !== undefined && origin !== undefined
        ? jobPath(origin, stage.session_id)
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
  useEffect(() => {
    for (const one of sends) {
      if (one.stage.phase !== "dispatched" || told.current.has(one.id)) continue;
      told.current.add(one.id);
      tell(one.stage, one.origin);
    }
  }, [
    sends,
  ]);

  const send = () => {
    if (mac === undefined || receiver === undefined || line.trim() === "") return;
    const words = line.trim();
    const hint = kind;
    setTrouble("");
    void dispatch(receiver, mac, {
      text: words,
      repository: named,
      ...(hint === undefined
        ? {}
        : {
            kind: hint,
          }),
    }).then((why) => {
      if (why === undefined) return;
      // The send is left in the box, as it was.
      setTrouble(why);
      setLine((was) => (was.trim() === "" ? words : `${was.trimEnd()}\n\n${words}`));
      setKind(hint);
    });
    setLine("");
    setKind(undefined);
  };

  const back = async (one: Send) => {
    const answer = await takeBack(one);
    if ("said" in answer) {
      setToast({
        text: answer.said,
        hue: "completed-failed",
      });
      later(() => setToast(undefined), TOAST_MS);
      return;
    }
    const { sent } = answer;
    setLine((was) => (was.trim() === "" ? sent.words : `${was.trimEnd()}\n\n${sent.words}`));
    setKind(sent.kind);
    if (sent.repository !== undefined) setRepository(sent.repository);
    if (sent.origin !== undefined) setPicked(sent.origin);
    window.requestAnimationFrame(() => document.getElementById(WORDS)?.focus());
  };

  const away = mac !== undefined && asleep(mac);

  return (
    <section className="pk-screen" aria-label="Operator">
      <Band title="Operator" onBack={() => go("/")} back="Home" />
      <div className="pk-body pk-home">
        <ul className="deck-list__rows" aria-label="Sends">
          {sends.map((one) => (
            <SendRow key={one.id} send={one} onTakeBack={(row) => void back(row)} />
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
            disabled={line.trim() === "" || mac === undefined || receiver === undefined}
            onClick={send}
          >
            Send
          </Button>
        </div>
        <div className="pk-composer__row">
          <div className="pk-composer__repository">
            <Select
              aria-label="Repository"
              value={named}
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
                key={one.machine.origin}
                role="radio"
                hue={asleep(one) ? "escalated" : "completed-success"}
                icon={asleep(one) ? Moon : undefined}
                checked={mac === one}
                onPress={() => setPicked(one.machine.origin)}
              >
                {asleep(one) ? `${nameOf(one.machine)}, asleep` : nameOf(one.machine)}
              </Chip>
            ))}
          </div>
        </div>
        {trouble === "" ? null : (
          <p className="pk-fact pk-composer__fact" role="alert">
            {trouble}
          </p>
        )}
        {away ? (
          <p className="pk-fact pk-composer__fact" role="status">
            {nameOf(mac.machine)} is not answering. A send waits in the list until it wakes.
          </p>
        ) : null}
      </footer>
    </section>
  );
}
