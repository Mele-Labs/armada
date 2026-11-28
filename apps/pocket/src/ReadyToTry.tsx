// A Ready to try card on the phone, in the Pictures first layout the owner picked (15 Nov 2026): what he approved above
// what was built, as wide as the phone, his words, a fold with what changed and where to look, then Looks good, Needs
// rework, Show me how and Remind me. The card is Fleet's `ThreadReadyToTry`, read from the Job's thread
// (`/api/sessions/thread`) only while one waits on him, and answered with `answer_ready_to_try`. Show me how lists the
// steps, since the acts they name run on the Mac.

import { useEffect, useState } from "react";
import { BadgeCheck, Clock, Reply } from "lucide-react";
import { Button } from "./kit/components/primitives/Button/Button";
import { CardFooter } from "./kit/components/primitives/Card/Card";
import {
  DropdownMenu,
  type DropdownMenuEntry,
} from "./kit/components/primitives/DropdownMenu/DropdownMenu";
import { SketchScene } from "./kit/components/compositions/SketchScene/SketchScene";
import { Textarea } from "./kit/components/primitives/Textarea/Textarea";
import type {
  AnswerReadyToTry,
  JobArtifact,
  JobThread,
  ReadyAnswer,
  ThreadReadyToTry,
} from "./wire";

import { firstPicture } from "./Artifact";
import { aspectOf, useArtifacts } from "./artifacts";
import { call, getJson, refusal } from "./client";
import { loadLists, macOf, onChange, start, useFleet } from "./fleet";
import { bodyOf } from "./jobState";
import { WithKeptStyles } from "./keptStyles";
import type { Machine } from "./machines";
import { Pictures, type Picture } from "./Pictures";
import { go, jobPath, tryPath } from "./router";
import { Band, Trouble } from "./ui";

/** How many of the thread's newest items are read for its cards: a card waiting on him is recent, or a reminder brought it back. */
const ITEMS = 200;

/** The Ready to try cards in the Job's newest items, read again on every change Fleet tells of the Job while `wanted`. */
export function useTryCards(
  machine: Machine | undefined,
  id: string,
  wanted: boolean,
): [
  readonly ThreadReadyToTry[] | undefined,
  () => void,
] {
  const [cards, setCards] = useState<readonly ThreadReadyToTry[] | undefined>(undefined);
  const [epoch, setEpoch] = useState(0);
  useEffect(() => {
    if (machine === undefined || !wanted) return;
    return onChange((origin, sessionId) => {
      if (origin === machine.origin && (sessionId === undefined || sessionId === id))
        setEpoch((was) => was + 1);
    });
  }, [
    machine?.origin,
    id,
    wanted,
  ]);
  useEffect(() => {
    if (machine === undefined || !wanted) return;
    let gone = false;
    getJson<JobThread>(
      machine,
      `/api/sessions/thread?session_id=${encodeURIComponent(id)}&limit=${String(ITEMS)}`,
    ).then(
      (thread) =>
        !gone &&
        setCards(
          thread.items.flatMap((item) =>
            item.kind === "ready_to_try"
              ? [
                  item,
                ]
              : [],
          ),
        ),
      () => undefined,
    );
    return () => {
      gone = true;
    };
  }, [
    machine?.origin,
    machine?.deviceId,
    id,
    wanted,
    epoch,
  ]);
  return [
    cards,
    () => setEpoch((was) => was + 1),
  ];
}

/** A shot the Job attached, read through `get_repository_file` as an address the page shows. */
function useShots(machine: Machine | undefined, id: string, paths: readonly string[]) {
  const [shots, setShots] = useState<readonly Picture[]>([]);
  const key = paths.join("\n");
  useEffect(() => {
    if (machine === undefined || paths.length === 0) return;
    let gone = false;
    void Promise.all(
      paths.map(async (path): Promise<Picture | undefined> => {
        try {
          const file = await getJson<{
            kind: string;
            bytes?: string;
            type?: string;
          }>(
            machine,
            `/api/sessions/repository_file?session_id=${encodeURIComponent(id)}&path=${encodeURIComponent(path)}`,
          );
          if (file.kind !== "picture" || file.bytes === undefined) return undefined;
          return {
            id: path,
            title: `Built: ${path.slice(path.lastIndexOf("/") + 1)}`,
            src: `data:${file.type ?? "image/png"};base64,${file.bytes}`,
          };
        } catch {
          return undefined;
        }
      }),
    ).then(
      (read) =>
        !gone &&
        setShots(
          read.flatMap((one) =>
            one === undefined
              ? []
              : [
                  one,
                ],
          ),
        ),
    );
    return () => {
      gone = true;
    };
  }, [
    machine?.origin,
    id,
    key,
  ]);
  return shots;
}

const DAY = new Intl.DateTimeFormat(undefined, {
  day: "numeric",
  month: "short",
});
const TIME = new Intl.DateTimeFormat(undefined, {
  hour: "2-digit",
  minute: "2-digit",
});

/** The Later ladder, as Bridge offers it: an hour, three hours, nine tomorrow, nine next Monday. */
export function laterChoices(now: Date): readonly {
  label: string;
  until: Date;
}[] {
  const nine = (days: number) => {
    const at = new Date(now);
    at.setDate(at.getDate() + days);
    at.setHours(9, 0, 0, 0);
    return at;
  };
  const toMonday = 7 - ((now.getDay() + 6) % 7);
  return [
    {
      label: "1 hour",
      until: new Date(now.getTime() + 3_600_000),
    },
    {
      label: "3 hours",
      until: new Date(now.getTime() + 3 * 3_600_000),
    },
    {
      label: "Tomorrow morning",
      until: nine(1),
    },
    {
      label: "Next week",
      until: nine(toMonday),
    },
  ];
}

/** The picture of what he approved: its drawing or first take, from the Job's artifacts. */
function approvedPicture(
  card: ThreadReadyToTry,
  artifacts: readonly JobArtifact[] | undefined,
): Picture | undefined {
  const held = artifacts?.find(
    (one) => one.id === card.approved?.artifact || one.title === card.approved?.artifact,
  );
  const picture = held === undefined ? undefined : firstPicture(held);
  if (picture === undefined || card.approved === undefined) return undefined;
  const on =
    card.approved.approved_at === undefined
      ? ""
      : ` · ${DAY.format(new Date(card.approved.approved_at))}`;
  return {
    ...picture,
    id: `approved:${card.approved.artifact}`,
    title: `You approved${on}`,
  };
}

/** One line for a card he answered, or that is not his to answer any more. */
function settledOf(card: ThreadReadyToTry): string | undefined {
  switch (card.state) {
    case "answered":
      return card.verdict === "needs_work" ? "Needs rework" : "Looks good";
    case "later":
      return card.later_until === undefined
        ? "Later"
        : `Back at ${TIME.format(new Date(card.later_until))}, ${DAY.format(new Date(card.later_until))}`;
    case "withdrawn":
      return "Withdrawn by the Job";
    case "replaced":
      return "A newer build replaced this one";
    case "open":
      return undefined;
  }
}

/** The card read whole: the screen it opens on from the thread or Needs you. */
export function TryCard({
  machine,
  id,
  card,
  reread,
  bare = false,
}: {
  machine: Machine;
  id: string;
  card: ThreadReadyToTry;
  reread: () => void;
  /** A row of the one Ready to try card To review draws, which carries the band and the frame. */
  bare?: boolean;
}) {
  const [artifacts] = useArtifacts(card.approved === undefined ? undefined : machine, id);
  const approved = approvedPicture(card, artifacts.artifacts);
  const shots = useShots(machine, id, card.pictures ?? []);
  const pictures: readonly Picture[] = [
    ...(approved === undefined
      ? []
      : [
          approved,
        ]),
    ...shots,
  ];
  const [opened, setOpened] = useState<number | undefined>(undefined);
  const [rework, setRework] = useState(false);
  const [how, setHow] = useState(false);
  const [words, setWords] = useState("");
  const [says, setSays] = useState("");
  const keep = `pocket:${machine.origin}:${id}:try:${card.id}`;
  const settled = settledOf(card);
  const open = card.state === "open";

  const answer = async (given: ReadyAnswer) => {
    setSays("");
    const body: AnswerReadyToTry = {
      session_id: id,
      branch: card.branch,
      tip: card.tip,
      answer: given,
    };
    try {
      await call(machine, "/api/sessions/ready_to_try/answer", {
        method: "POST",
        body,
      });
      setRework(false);
      setWords("");
      reread();
      void loadLists(machine.origin);
    } catch (why) {
      setSays(bodyOf(why).message ?? refusal(why, machine));
    }
  };

  const ladder = laterChoices(new Date());
  const later: DropdownMenuEntry[] = ladder.map((one) => ({
    kind: "item",
    id: one.label,
    label: one.label,
    hint: `Back at ${TIME.format(one.until)}, ${DAY.format(one.until)}`,
  }));

  return (
    <article className={bare ? "pk-try pk-try--bare" : "pk-try"} aria-label={card.about}>
      {bare ? null : (
        <header className="pk-try__band">
          <span className="pk-try__chip">Ready to try</span>
          <span className="pk-try__at">{TIME.format(new Date(card.at))}</span>
        </header>
      )}
      <h2 className="pk-try__title">{card.about}</h2>
      {pictures.length === 0 ? null : (
        <WithKeptStyles machine={machine}>
          <div className="pk-try__pictures">
            {pictures.map((picture, at) => (
              <figure key={picture.id} className="pk-try__picture">
                <figcaption
                  className="pk-try__caption"
                  data-side={picture.id.startsWith("approved:") ? "approved" : "built"}
                >
                  {picture.id.startsWith("approved:") ? picture.title : "Built · in the preview"}
                </figcaption>
                <button
                  type="button"
                  className="pk-art__picture"
                  aria-label={`Open ${picture.title} full screen`}
                  style={
                    "scene" in picture
                      ? {
                          aspectRatio: String(aspectOf(picture.scene)),
                        }
                      : undefined
                  }
                  onClick={() => setOpened(at)}
                >
                  {"scene" in picture ? (
                    <SketchScene scene={picture.scene} title={picture.title} preview />
                  ) : (
                    <img className="pk-art__shot" src={picture.src} alt={picture.title} />
                  )}
                </button>
              </figure>
            ))}
          </div>
          {opened === undefined ? null : (
            <Pictures
              pictures={pictures}
              at={opened}
              onAt={setOpened}
              onClose={() => setOpened(undefined)}
            />
          )}
        </WithKeptStyles>
      )}
      {card.words === undefined ? null : (
        <blockquote className="pk-try__words">
          <span className="pk-try__words-label">Your words</span>
          {card.words}
        </blockquote>
      )}
      <details className="pk-try__fold">
        <summary>What changed and where to look</summary>
        <dl className="pk-try__facts">
          <dt>What changed</dt>
          <dd>{card.about}</dd>
          {card.look === undefined ? null : (
            <>
              <dt>Where to look</dt>
              <dd>{card.look}</dd>
            </>
          )}
          <dt>Build</dt>
          <dd>
            {card.in_build
              ? "Your preview holds it"
              : "Not in your preview yet: press Refresh preview on your Mac"}
          </dd>
        </dl>
      </details>
      {how && (card.steps ?? []).length > 0 ? (
        <ol className="pk-try__steps" aria-label="Steps on your Mac">
          {(card.steps ?? []).map((step, at) => (
            <li key={`${String(at)} ${step.say}`}>{step.say}</li>
          ))}
        </ol>
      ) : null}
      {settled === undefined ? null : (
        <p className="pk-try__settled" data-state={card.state} data-verdict={card.verdict}>
          {card.state === "later" ? (
            <Clock size={16} aria-hidden="true" />
          ) : card.verdict === "needs_work" ? (
            <Reply size={16} aria-hidden="true" />
          ) : (
            <BadgeCheck size={16} aria-hidden="true" />
          )}
          {settled}
        </p>
      )}
      {rework && open ? (
        <Textarea
          aria-label="What needs rework"
          rows={3}
          autoFocus
          keep={`${keep}:rework`}
          value={words}
          onChange={(event) => setWords(event.currentTarget.value)}
        />
      ) : null}
      <Trouble says={says} />
      {!open && card.state !== "later" ? null : (
        <CardFooter className="pk-try__foot" role="group" aria-label="Answers">
          {rework ? (
            <>
              <Button
                size="sm"
                variant="primary"
                disabled={words.trim() === ""}
                onClick={() =>
                  void answer({
                    kind: "needs_rework",
                    words: words.trim(),
                  })
                }
              >
                Send
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setRework(false)}>
                Cancel
              </Button>
            </>
          ) : (
            <>
              <Button
                size="sm"
                variant="primary"
                onClick={() =>
                  void answer({
                    kind: "looks_good",
                  })
                }
              >
                Looks good
              </Button>
              <Button size="sm" variant="secondary" onClick={() => setRework(true)}>
                Needs rework
              </Button>
              {(card.steps ?? []).length === 0 ? null : (
                <Button size="sm" variant="ghost" aria-expanded={how} onClick={() => setHow(!how)}>
                  Show me how
                </Button>
              )}
              <DropdownMenu
                triggerLabel="Remind me"
                align="end"
                entries={later}
                onSelect={(label) => {
                  const until = ladder.find((one) => one.label === label)?.until;
                  if (until !== undefined)
                    void answer({
                      kind: "remind",
                      until: until.toISOString(),
                    });
                }}
              />
            </>
          )}
        </CardFooter>
      )}
    </article>
  );
}

export function TryView({
  origin,
  id,
  card: wanted,
}: {
  origin: string;
  id: string;
  card: string;
}) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [cards, reread] = useTryCards(machine, id, true);
  const card = cards?.find((one) => one.id === wanted);
  return (
    <section className="pk-screen" aria-label={card?.about ?? "Ready to try"}>
      <Band title="Ready to try" onBack={() => go(jobPath(origin, id))} back="Back to the Job" />
      <div className="pk-body pk-art__body">
        {cards !== undefined && card === undefined ? (
          <Trouble says="That Job holds no such card." />
        ) : null}
        {machine === undefined || card === undefined ? null : (
          <TryCard machine={machine} id={id} card={card} reread={reread} />
        )}
      </div>
    </section>
  );
}

/** The card at the foot of the thread while one waits on him: what was built, its first picture, and Review. */
export function TryLine({
  machine,
  id,
  card,
}: {
  machine: Machine;
  id: string;
  card: ThreadReadyToTry;
}) {
  const open = () => go(tryPath(machine.origin, id, card.id));
  return (
    <article className="pk-art-card pk-art-card--try" aria-label={`Ready to try: ${card.about}`}>
      <header className="pk-art-card__band">
        <span className="pk-try__chip">Ready to try</span>
        <span className="pk-art-card__title">{card.about}</span>
      </header>
      {card.words === undefined ? null : <p className="pk-art-card__for">“{card.words}”</p>}
      <CardFooter className="pk-art-card__foot">
        <Button size="sm" variant="primary" onClick={open}>
          Review
        </Button>
      </CardFooter>
    </article>
  );
}
