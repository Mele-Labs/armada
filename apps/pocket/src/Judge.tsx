// The Judge on the phone: what to judge, which judge type, an optional line to verify, and the readings with their notes. A
// press asks Fleet for one read-only reading (`POST /api/sessions/judge`); a note is sent to the agent or dismissed by the
// person alone. The order is native Bridge's (`crates/bridge-native/src/jobs/judging.rs`): what, then the type, then the
// reading. `docs/concepts/judge.md`.

import { useEffect, useRef, useState } from "react";
import { Button } from "./kit/components/primitives/Button/Button";
import { Skeleton } from "./kit/components/primitives/Skeleton/Skeleton";
import { Textarea } from "./kit/components/primitives/Textarea/Textarea";
import type {
  AskJudge,
  JudgeArtifact,
  JudgeNote,
  JudgeReading,
  JudgeType,
  JudgeTypes,
  SessionRecord,
} from "./wire";

import { call, getJson } from "./client";
import { macOf, start, useFleet } from "./fleet";
import { attempt, useJob } from "./jobState";
import type { Machine } from "./machines";
import { go, jobPath } from "./router";
import { Band, Trouble } from "./ui";

/** What can be judged on a Job: the Job, and each pull request it holds. */
export function targetsOf(record: SessionRecord): JudgeArtifact[] {
  const pulls = record.attachments
    .filter((one) => one.kind === "pr" || one.kind === "pull_request")
    .map((one) => one.target.replace(/^#/, ""))
    .filter((number, at, all) => /^\d+$/.test(number) && all.indexOf(number) === at);
  return [
    {
      kind: "job",
      id: record.id,
      title: record.title ?? "This Job",
    },
    ...pulls.map(
      (number): JudgeArtifact => ({
        kind: "pull_request",
        id: number,
        title: `#${number}`,
      }),
    ),
  ];
}

const sameArtifact = (a: JudgeArtifact, b: JudgeArtifact) => a.kind === b.kind && a.id === b.id;

/** The note's lines, each only where Fleet gave it. */
function Note({
  note,
  onSend,
  onDismiss,
  busy,
}: {
  note: JudgeNote;
  onSend: () => void;
  onDismiss: () => void;
  busy: boolean;
}) {
  const lines: [
    string,
    string | undefined,
  ][] = [
    [
      "Expected",
      note.expected,
    ],
    [
      "Found",
      note.found,
    ],
    [
      "Impact",
      note.impact,
    ],
  ];
  return (
    <li className="pk-judge-note" data-settled={note.settled?.how}>
      <p className="pk-judge-note__claim">{note.claim}</p>
      {note.where === undefined ? null : <p className="pk-judge-note__where">{note.where}</p>}
      {lines.map(([name, text]) =>
        text === undefined ? null : (
          <p key={name} className="pk-judge-note__line">
            <span className="pk-judge-note__name">{name}</span> {text}
          </p>
        ),
      )}
      {note.settled !== undefined ? (
        <p className="pk-judge-note__settled">
          {note.settled.how === "sent" ? "Sent to the agent" : "Dismissed"}
        </p>
      ) : (
        <div className="pk-judge-note__acts" role="group" aria-label="Note">
          <Button variant="primary" size="sm" disabled={busy} onClick={onSend}>
            Send to agent
          </Button>
          <Button variant="ghost" size="sm" disabled={busy} onClick={onDismiss}>
            Dismiss
          </Button>
        </div>
      )}
    </li>
  );
}

/** One reading: going, failed with Fleet's reason, clean, or its notes. */
export function Reading({
  reading,
  act,
}: {
  reading: JudgeReading;
  act: (path: string, body: object) => Promise<boolean>;
}) {
  const [busy, setBusy] = useState(false);
  const settle = async (path: string, note: JudgeNote) => {
    setBusy(true);
    await act(path, {
      reading: reading.id,
      note: note.id,
    });
    setBusy(false);
  };
  const as = reading.judge_name === undefined ? "" : ` as ${reading.judge_name}`;
  return (
    <section className="pk-judge-reading" aria-label={`Judge of ${reading.artifact.title}`}>
      <h3 className="pk-judge-reading__title">
        {reading.artifact.title}
        <span className="pk-judge-reading__as">{as}</span>
      </h3>
      {reading.state === "reading" ? (
        <p className="pk-judge-reading__state" role="status">
          Reading
        </p>
      ) : null}
      {reading.state === "failed" ? (
        <p className="pk-judge-reading__failed" role="alert">
          {reading.failed ?? "The Judge ended without notes that held up."}
        </p>
      ) : null}
      {reading.state === "done" && reading.notes.length === 0 ? (
        <p className="pk-judge-reading__state">Nothing missed</p>
      ) : null}
      {reading.notes.length === 0 ? null : (
        <ul className="pk-judge-notes">
          {reading.notes.map((note) => (
            <Note
              key={note.id}
              note={note}
              busy={busy}
              onSend={() => void settle("/api/sessions/judge/send", note)}
              onDismiss={() => void settle("/api/sessions/judge/dismiss", note)}
            />
          ))}
        </ul>
      )}
    </section>
  );
}

/** The types a repository offers, read once for the Job. */
function useJudgeTypes(
  machine: Machine | undefined,
  repository: string | undefined,
): [
  JudgeType[] | undefined,
  string | undefined,
] {
  const [types, setTypes] = useState<JudgeType[] | undefined>(undefined);
  const [error, setError] = useState<string | undefined>(undefined);
  useEffect(() => {
    if (machine === undefined || repository === undefined) return;
    let gone = false;
    getJson<JudgeTypes>(machine, `/api/judge_types/${encodeURIComponent(repository)}`).then(
      (read) => {
        if (!gone) setTypes(read.types);
      },
      (why: unknown) => {
        if (!gone) setError(why instanceof Error ? why.message : "The judge types did not load.");
      },
    );
    return () => {
      gone = true;
    };
  }, [
    machine?.origin,
    repository,
  ]);
  return [
    types,
    error,
  ];
}

export function JudgeView({ origin, id }: { origin: string; id: string }) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [read, reread] = useJob(machine, id);
  const record = read.thread?.session;
  const [types, typesError] = useJudgeTypes(machine, record?.manifest_id);
  const [target, setTarget] = useState(0);
  const [type, setType] = useState<string | undefined>(undefined);
  const [verify, setVerify] = useState("");
  const [says, setSays] = useState<string | null>(null);
  const [asking, setAsking] = useState(false);
  const back = () => go(jobPath(origin, id));

  // A reading's notes are opened the moment they are on the screen, once.
  const opened = useRef(new Set<string>());
  const readings = [
    ...(record?.judge_readings ?? []),
  ].reverse();
  useEffect(() => {
    if (machine === undefined) return;
    for (const one of readings) {
      if (one.state !== "done" || one.read || opened.current.has(one.id)) continue;
      opened.current.add(one.id);
      void call(machine, "/api/sessions/judge/read", {
        method: "POST",
        body: {
          session_id: id,
          reading: one.id,
        },
      }).then(reread, () => undefined);
    }
  });

  if (machine === undefined || record === undefined) {
    return (
      <section className="pk-screen" aria-label="Judge">
        <Band title="Judge" onBack={back} back="Back to the Job" />
        <div className="pk-body">
          {read.error === undefined ? <Skeleton width="60%" /> : <Trouble says={read.error} />}
        </div>
      </section>
    );
  }

  const targets = targetsOf(record);
  const chosen = targets[Math.min(target, targets.length - 1)]!;
  const typeId = type ?? types?.[0]?.id;

  const ask = async () => {
    setAsking(true);
    const body: AskJudge = {
      session_id: id,
      artifact: chosen,
      prompt: verify.trim(),
      ...(typeId === undefined
        ? {}
        : {
            judge: typeId,
          }),
    };
    const failed = await attempt(async () => {
      await call(machine, "/api/sessions/judge", {
        method: "POST",
        body,
      });
      reread();
    }, machine);
    setSays(failed);
    if (failed === null) setVerify("");
    setAsking(false);
  };
  const act = async (path: string, rest: object): Promise<boolean> => {
    const failed = await attempt(async () => {
      await call(machine, path, {
        method: "POST",
        body: {
          session_id: id,
          ...rest,
        },
      });
      reread();
    }, machine);
    setSays(failed);
    return failed === null;
  };

  return (
    <section className="pk-screen" aria-label="Judge">
      <Band title="Judge" onBack={back} back="Back to the Job" />
      <div className="pk-body pk-judge">
        <div className="pk-judge-ask">
          <h2 className="pk-judge-head">What to judge</h2>
          <div className="pk-judge-chips" role="radiogroup" aria-label="What to judge">
            {targets.map((one, at) => (
              <Button
                key={`${one.kind}:${one.id}`}
                variant={sameArtifact(one, chosen) ? "primary" : "secondary"}
                size="sm"
                role="radio"
                aria-checked={sameArtifact(one, chosen)}
                onClick={() => setTarget(at)}
              >
                {one.kind === "job" ? "The Job" : `Pull request ${one.title}`}
              </Button>
            ))}
          </div>
          <h2 className="pk-judge-head">Judge type</h2>
          {types === undefined && typesError === undefined ? <Skeleton width="70%" /> : null}
          {typesError === undefined ? null : <Trouble says={typesError} />}
          <div className="pk-judge-chips" role="radiogroup" aria-label="Judge type">
            {(types ?? []).map((one) => (
              <Button
                key={one.id}
                variant={one.id === typeId ? "primary" : "secondary"}
                size="sm"
                role="radio"
                aria-checked={one.id === typeId}
                onClick={() => setType(one.id)}
              >
                {one.name}
              </Button>
            ))}
          </div>
          <Textarea
            aria-label="Verify that"
            placeholder="Verify that (optional)"
            rows={2}
            keep={`pocket:${origin}:${id}:judge`}
            value={verify}
            onChange={(event) => setVerify(event.currentTarget.value)}
          />
          <Button
            variant="primary"
            disabled={asking || typeId === undefined}
            onClick={() => void ask()}
          >
            Judge
          </Button>
          <Trouble says={says ?? ""} />
        </div>
        {readings.map((one) => (
          <Reading key={one.id} reading={one} act={act} />
        ))}
      </div>
    </section>
  );
}
