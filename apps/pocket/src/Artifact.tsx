// One artifact read whole on the phone, the whole screen with only Back: its parts in the order the Job added them,
// each answered on its own as native Bridge answers them (`crates/bridge-native/src/job_view/parts_card.rs`). Words, a
// drawing and a page take Approve or Ask for changes; takes are a pick, each take's picture as wide as the phone and
// opened full screen on a tap; buttons act when pressed. A page is the sealed page Fleet serves, so its own votes and
// actions work here too. `ArtifactCard` is the card at the foot of the thread while it asks.

import { useEffect, useMemo, useRef, useState } from "react";
import { BadgeCheck, Check, MessageSquare, Reply } from "lucide-react";
import { ApprovalAnswers } from "./kit/components/compositions/SessionThread/SketchRow";
import { Button } from "./kit/components/primitives/Button/Button";
import { CardFooter } from "./kit/components/primitives/Card/Card";
import { Prose } from "./kit/components/primitives/Prose/Prose";
import { SketchScene } from "./kit/components/compositions/SketchScene/SketchScene";
import { Textarea } from "./kit/components/primitives/Textarea/Textarea";
import { Tooltip } from "./kit/components/primitives/Tooltip/Tooltip";
import type {
  SketchAnswer,
  SketchSettled,
} from "./kit/components/compositions/SessionThread/sketch-answer";
import type { ArtifactPart, HostToPage, JobArtifact, PageMessage, PageToHost } from "./wire";

import {
  aspectOf,
  asks,
  markSeen,
  newestOf,
  opensAsk,
  pageAct,
  partActs,
  partsOf,
  pickedTitle,
  readPage,
  sceneOfPart,
  useArtifacts,
} from "./artifacts";
import { macOf, start, useFleet } from "./fleet";
import { WithKeptStyles } from "./keptStyles";
import type { Machine } from "./machines";
import { Pictures, type Picture } from "./Pictures";
import { artifactPath, go, jobPath } from "./router";
import { Trouble } from "./ui";
import {
  BandCard,
  revisionSaid,
  Well,
  type BandFacts,
} from "./kit/components/compositions/BandCard/BandCard";
import { DraftText } from "./kit/components/compositions/SessionThread/DraftRow";
import { Reader } from "./Reader";
import { actsOn, attempt, useJob } from "./jobState";
import { draftsOf, sessionOfThread } from "./thread";

const KIND: Record<ArtifactPart["kind"], string> = {
  drawing: "Drawing",
  words: "Words",
  takes: "Takes",
  buttons: "Buttons",
  page: "Page",
  prototype: "Prototype",
};

/** How a part he answered stands, as `ApprovalAnswers` draws it. */
function settledOf(part: ArtifactPart): SketchSettled | undefined {
  if (part.state !== "approved" && part.state !== "sent_back") return undefined;
  return {
    how: part.state,
    at: part.answer?.at ?? "",
    ...(part.answer?.words === undefined
      ? {}
      : {
          words: part.answer.words,
        }),
  };
}

/** A drawing as wide as the phone, its height from the scene's shape, which a tap opens full screen. */
export function PictureBox({
  picture,
  onOpen,
  picked,
}: {
  picture: Picture;
  onOpen: () => void;
  picked?: boolean;
}) {
  return (
    <button
      type="button"
      className="pk-art__picture"
      data-picked={picked === true ? "true" : undefined}
      aria-label={`Open ${picture.title} full screen`}
      style={
        "scene" in picture
          ? {
              aspectRatio: String(aspectOf(picture.scene)),
            }
          : undefined
      }
      onClick={onOpen}
    >
      {"scene" in picture ? (
        <SketchScene scene={picture.scene} title={picture.title} preview />
      ) : (
        <img className="pk-art__shot" src={picture.src} alt={picture.title} />
      )}
    </button>
  );
}

type Opened = {
  pictures: readonly Picture[];
  at: number;
  part?: string;
};

type Acts = ReturnType<typeof partActs>;

function Answers({
  part,
  keep,
  acts,
  onSaid,
}: {
  part: ArtifactPart;
  keep: string;
  acts: Acts;
  onSaid: (said: string) => void;
}) {
  const settled = settledOf(part);
  if (!part.asks || (!opensAsk(part) && settled === undefined)) return null;
  return (
    <ApprovalAnswers
      id={part.id}
      keep={keep}
      open={opensAsk(part)}
      {...(settled === undefined
        ? {}
        : {
            settled,
          })}
      onAnswer={(_, given: SketchAnswer) =>
        void (
          "text" in given
            ? acts.answer(part, "sent_back", {
                words: given.text,
              })
            : acts.answer(part, "approved")
        ).then((refused) => onSaid(refused ?? ""))
      }
    />
  );
}

function TakesPart({
  part,
  keep,
  acts,
  onOpen,
  chosen,
  onChoose,
  onSaid,
}: {
  part: Extract<
    ArtifactPart,
    {
      kind: "takes";
    }
  >;
  keep: string;
  acts: Acts;
  onOpen: (opened: Opened) => void;
  chosen: string | undefined;
  onChoose: (take: string) => void;
  onSaid: (said: string) => void;
}) {
  const [words, setWords] = useState("");
  const [notes, setNotes] = useState<Record<string, string>>({});
  const [noting, setNoting] = useState<string | undefined>(undefined);
  const open = opensAsk(part) && part.form === "pick";
  const pictures: Picture[] = part.takes.flatMap((take) => {
    const scene = sceneOfPart(take.scene);
    return scene === undefined
      ? []
      : [
          {
            id: take.id,
            title: take.title,
            scene,
          },
        ];
  });
  const picked = part.answer?.take;
  const send = () => {
    if (chosen === undefined) return;
    const kept = Object.fromEntries(Object.entries(notes).filter(([, text]) => text.trim() !== ""));
    void acts
      .answer(part, "picked", {
        take: chosen,
        ...(words.trim() === ""
          ? {}
          : {
              words: words.trim(),
            }),
        ...(Object.keys(kept).length === 0
          ? {}
          : {
              notes: kept,
            }),
      })
      .then((refused) => onSaid(refused ?? ""));
  };
  return (
    <>
      {part.question === undefined ? null : <p className="pk-art__question">{part.question}</p>}
      <ol className="pk-art__takes">
        {part.takes.map((take) => {
          const at = pictures.findIndex((one) => one.id === take.id);
          const mine = (open ? chosen : picked) === take.id;
          return (
            <li key={take.id} className="pk-art__take" data-picked={mine ? "true" : undefined}>
              <div className="pk-art__take-head">
                <span className="pk-art__take-title">{take.title}</span>
                {mine ? (
                  <Tooltip label={open ? "Your pick, not sent yet" : "Your pick"}>
                    <span className="pk-art__mark" role="img" aria-label="Picked">
                      <Check size={14} strokeWidth={2.5} aria-hidden="true" />
                    </span>
                  </Tooltip>
                ) : null}
              </div>
              {at < 0 ? null : (
                <PictureBox
                  picture={pictures[at]!}
                  picked={mine}
                  onOpen={() =>
                    onOpen({
                      pictures,
                      at,
                      ...(open
                        ? {
                            part: part.id,
                          }
                        : {}),
                    })
                  }
                />
              )}
              {part.answer?.notes?.[take.id] === undefined ? null : (
                <blockquote className="pk-art__said">{part.answer.notes[take.id]}</blockquote>
              )}
              {!open ? null : (
                <div className="pk-art__take-acts">
                  <Button
                    size="sm"
                    variant={mine ? "tonal" : "secondary"}
                    aria-pressed={mine}
                    onClick={() => onChoose(take.id)}
                  >
                    {mine ? <Check size={14} aria-hidden="true" /> : null}
                    {mine ? "Picked" : "Pick"}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    aria-expanded={noting === take.id}
                    onClick={() => setNoting(noting === take.id ? undefined : take.id)}
                  >
                    <MessageSquare size={14} aria-hidden="true" />
                    Note
                  </Button>
                </div>
              )}
              {open && (noting === take.id || (notes[take.id] ?? "") !== "") ? (
                <Textarea
                  aria-label={`Note on ${take.title}`}
                  rows={2}
                  keep={`${keep}:note:${take.id}`}
                  value={notes[take.id] ?? ""}
                  onChange={(event) => {
                    const text = event.currentTarget.value;
                    setNotes((was) => ({
                      ...was,
                      [take.id]: text,
                    }));
                  }}
                />
              ) : null}
            </li>
          );
        })}
      </ol>
      {!open ? (
        picked === undefined ? null : (
          <p className="pk-art__settled" data-how="picked">
            <BadgeCheck size={16} strokeWidth={2} aria-hidden="true" />
            Picked {pickedTitle(part)}
            {part.answer?.words === undefined ? null : (
              <span className="pk-art__settled-words">{part.answer.words}</span>
            )}
          </p>
        )
      ) : (
        <>
          <Textarea
            aria-label="Words with your pick"
            rows={2}
            keep={`${keep}:words`}
            value={words}
            onChange={(event) => setWords(event.currentTarget.value)}
          />
          <CardFooter role="group" aria-label="Answers">
            <Button size="sm" variant="primary" disabled={chosen === undefined} onClick={send}>
              Send pick
            </Button>
          </CardFooter>
        </>
      )}
    </>
  );
}

/** A page, sealed as Fleet serves it: its kit's presses and votes come here, are made as Fleet's calls, and go back. */
function PagePart({
  machine,
  id,
  artifact,
  part,
  reread,
}: {
  machine: Machine;
  id: string;
  artifact: JobArtifact;
  part: Extract<
    ArtifactPart,
    {
      kind: "page";
    }
  >;
  reread: () => void;
}) {
  const newest = part.revision;
  const [revision, setRevision] = useState(newest);
  const [html, setHtml] = useState<string | undefined>(undefined);
  const [says, setSays] = useState("");
  const frame = useRef<HTMLIFrameElement>(null);
  // A joined page is answered under its own path; a page alone is the artifact.
  const page = (artifact.parts ?? []).length === 0 ? artifact.id : part.path;
  useEffect(
    () => setRevision(newest),
    [
      newest,
    ],
  );
  useEffect(() => {
    let gone = false;
    readPage(machine, id, page, revision).then(
      (text) => !gone && setHtml(text),
      () => !gone && setSays("The Mac did not send this page."),
    );
    return () => {
      gone = true;
    };
  }, [
    machine.origin,
    id,
    page,
    revision,
  ]);
  useEffect(() => {
    const heard = (event: MessageEvent) => {
      if (event.source !== frame.current?.contentWindow) return;
      const said = event.data as PageMessage<PageToHost>;
      if (said?.armada !== 1 || (said.kind !== "press" && said.kind !== "vote")) return;
      void pageAct(machine, id, said, reread).then((back: PageMessage<HostToPage>) =>
        frame.current?.contentWindow?.postMessage(back, "*"),
      );
    };
    window.addEventListener("message", heard);
    return () => window.removeEventListener("message", heard);
  }, [
    machine.origin,
    id,
  ]);
  return (
    <>
      {newest < 2 ? null : (
        <label className="pk-art__revision">
          <span>Revision</span>
          <select
            value={revision}
            onChange={(event) => setRevision(Number(event.currentTarget.value))}
          >
            {Array.from(
              {
                length: newest,
              },
              (_, at) => newest - at,
            ).map((number) => (
              <option key={number} value={number}>
                {number === newest ? `${String(number)}, newest` : String(number)}
              </option>
            ))}
          </select>
        </label>
      )}
      <Trouble says={says} />
      {html === undefined ? (
        <div className="pk-art__page pk-skeleton" aria-hidden="true" />
      ) : (
        <iframe
          ref={frame}
          className="pk-art__page"
          title={artifact.title}
          sandbox="allow-scripts"
          srcDoc={html}
        />
      )}
    </>
  );
}

function PartBlock({
  machine,
  id,
  artifact,
  part,
  keep,
  acts,
  onOpen,
  chosen,
  onChoose,
  reread,
}: {
  machine: Machine;
  id: string;
  artifact: JobArtifact;
  part: ArtifactPart;
  keep: string;
  acts: Acts;
  onOpen: (opened: Opened) => void;
  chosen: string | undefined;
  onChoose: (take: string) => void;
  reread: () => void;
}) {
  const [says, setSays] = useState("");
  const [pressed, setPressed] = useState<readonly string[]>([]);
  const tag = part.asks
    ? opensAsk(part)
      ? "Asks you"
      : part.state === "withdrawn"
        ? "Withdrawn"
        : part.state === "sent_back"
          ? "Sent back"
          : part.state === "picked"
            ? "Picked"
            : "Approved"
    : part.kind === "buttons"
      ? "Acts when pressed"
      : "To read";
  return (
    <section
      className="pk-art__part"
      data-kind={part.kind}
      data-asks={opensAsk(part) ? "true" : undefined}
      aria-label={KIND[part.kind]}
    >
      <header className="pk-art__caption">
        <span className="pk-art__kind">{KIND[part.kind]}</span>
        <span className="pk-art__tag" data-state={opensAsk(part) ? "open" : part.state}>
          {part.state === "sent_back" ? <Reply size={12} aria-hidden="true" /> : null}
          {tag}
        </span>
      </header>
      {part.kind === "words" ? <Prose text={part.text} /> : null}
      {part.kind === "drawing"
        ? (() => {
            const scene = sceneOfPart(part.scene);
            if (scene === undefined) return null;
            const picture = {
              id: part.id,
              title: artifact.title,
              scene,
            };
            return (
              <PictureBox
                picture={picture}
                onOpen={() =>
                  onOpen({
                    pictures: [
                      picture,
                    ],
                    at: 0,
                  })
                }
              />
            );
          })()
        : null}
      {part.kind === "takes" ? (
        <TakesPart
          part={part}
          keep={keep}
          acts={acts}
          onOpen={onOpen}
          chosen={chosen}
          onChoose={onChoose}
          onSaid={setSays}
        />
      ) : null}
      {part.kind === "buttons" ? (
        <div className="pk-art__buttons" role="group" aria-label="Buttons">
          {part.actions.map((action) => {
            const sent = pressed.includes(action.id);
            return (
              <Tooltip key={action.id} label={action.item ?? action.brief ?? action.label}>
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={sent}
                  onClick={() => {
                    setPressed((was) => [
                      ...was,
                      action.id,
                    ]);
                    void acts.press(part, action).then((refused) => {
                      setSays(refused ?? "");
                      if (refused !== null)
                        setPressed((was) => was.filter((one) => one !== action.id));
                    });
                  }}
                >
                  {sent ? <Check size={14} aria-hidden="true" /> : null}
                  {action.label}
                </Button>
              </Tooltip>
            );
          })}
        </div>
      ) : null}
      {part.kind === "page" ? (
        <PagePart machine={machine} id={id} artifact={artifact} part={part} reread={reread} />
      ) : null}
      {part.kind === "prototype" ? <Prose text="This prototype opens in the native app." /> : null}
      {part.kind === "takes" || part.kind === "buttons" ? null : (
        <Answers part={part} keep={keep} acts={acts} onSaid={setSays} />
      )}
      <Trouble says={says} />
    </section>
  );
}

export function ArtifactView({
  origin,
  id,
  artifact: wanted,
}: {
  origin: string;
  id: string;
  artifact: string;
}) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [read, reread] = useArtifacts(machine, id);
  const artifact = read.artifacts?.find((one) => one.id === wanted);
  const [opened, setOpened] = useState<Opened | undefined>(undefined);
  const [chosen, setChosen] = useState<Record<string, string>>({});
  const acts = useMemo(
    () =>
      machine === undefined || artifact === undefined
        ? undefined
        : partActs(machine, id, artifact, reread),
    [
      machine,
      id,
      artifact,
    ],
  );
  const newest = artifact === undefined ? 0 : newestOf(artifact);
  useEffect(() => {
    if (machine !== undefined && artifact !== undefined) markSeen(machine, id, artifact, newest);
  }, [
    machine?.origin,
    id,
    artifact?.id,
    newest,
  ]);
  const title = artifact?.title ?? wanted.replace(/^(sketch|draft):/, "");
  const keep = `pocket:${origin}:${id}:artifact:${wanted}`;
  // A draft that holds no parts is its text, which the Job's thread keeps as the row that proposed it.
  const bare = artifact?.form === "draft" && partsOf(artifact).length === 0;
  const [thread, rereadJob] = useJob(bare ? machine : undefined, id);
  const session = thread.thread === undefined ? undefined : sessionOfThread(thread.thread);
  const draft =
    !bare || session === undefined
      ? undefined
      : [
          ...session.rows,
        ]
          .reverse()
          .find((row) => row.kind === "draft" && row.title === artifact?.title);
  // Its answers, while the Mac holds it as an item waiting on him: the same ones the thread's card gives.
  const open =
    session === undefined || draft?.kind !== "draft"
      ? undefined
      : draftsOf(session).find((one) => one.id === draft.id);
  const [says, setSays] = useState("");
  const answer = (
    body:
      | {
          choice: number;
        }
      | {
          text: string;
        },
  ) => {
    if (machine === undefined || open === undefined) return;
    setSays("");
    void attempt(
      () =>
        actsOn(machine, rereadJob).answerWaiting({
          session_id: id,
          item_id: `draft:${open.id}`,
          ...body,
        }),
      machine,
    ).then((refused) => setSays(refused ?? ""));
  };
  const last = artifact?.revisions[artifact.revisions.length - 1];
  const facts: BandFacts = {
    form: artifact?.form ?? "page",
    title,
    stand: artifact?.state ?? "reading",
    words: artifact?.words,
    ...(last === undefined
      ? {}
      : {
          revision: {
            number: last.number,
            said: revisionSaid(last.number, last.by, last.at),
          },
        }),
    ...(last?.by === "owner"
      ? {
          yours: `r${String(last.number)} is your edit`,
        }
      : {}),
    forLine: artifact?.for_line,
  };
  return (
    <Reader
      facts={facts}
      keep={keep}
      onBack={() => go(jobPath(origin, id))}
      {...(artifact === undefined
        ? {}
        : {
            revisions: artifact.revisions,
            shown: newest,
          })}
      {...(open === undefined
        ? {}
        : {
            acts: {
              approve: () =>
                answer({
                  choice: 0,
                }),
              ask: (text: string) =>
                answer({
                  text,
                }),
              ...(open.proposed === true
                ? {
                    dismiss: () =>
                      answer({
                        choice: 1,
                      }),
                  }
                : {}),
            },
          })}
      says={says === "" ? undefined : says}
    >
      {read.error === undefined || read.artifacts !== undefined ? null : (
        <Trouble says={read.error} />
      )}
      {read.artifacts !== undefined && artifact === undefined ? (
        <Trouble says="That Job holds no such artifact." />
      ) : null}
      {draft?.kind === "draft" ? <DraftText text={draft.text} /> : null}
      {machine === undefined || artifact === undefined || acts === undefined ? null : (
        <WithKeptStyles machine={machine}>
          {partsOf(artifact).map((part) => (
            <PartBlock
              key={`${part.id} ${part.drawn}`}
              machine={machine}
              id={id}
              artifact={artifact}
              part={part}
              keep={`${keep}:${part.id}`}
              acts={acts}
              onOpen={setOpened}
              chosen={chosen[part.id]}
              onChoose={(take) =>
                setChosen((was) => ({
                  ...was,
                  [part.id]: take,
                }))
              }
              reread={reread}
            />
          ))}
          {opened === undefined ? null : (
            <Pictures
              pictures={opened.pictures}
              at={opened.at}
              onAt={(at) =>
                setOpened({
                  ...opened,
                  at,
                })
              }
              {...(opened.part === undefined
                ? {}
                : {
                    ...(chosen[opened.part] === undefined
                      ? {}
                      : {
                          picked: chosen[opened.part],
                        }),
                    onPick: (take: string) => {
                      setChosen((was) => ({
                        ...was,
                        [opened.part!]: take,
                      }));
                      setOpened(undefined);
                    },
                  })}
              onClose={() => setOpened(undefined)}
            />
          )}
        </WithKeptStyles>
      )}
    </Reader>
  );
}

/** The first picture an artifact holds: its drawing, or its first take's. */
export function firstPicture(artifact: JobArtifact): Picture | undefined {
  for (const part of partsOf(artifact)) {
    if (part.kind === "drawing") {
      const scene = sceneOfPart(part.scene);
      if (scene !== undefined)
        return {
          id: part.id,
          title: artifact.title,
          scene,
        };
    }
    if (part.kind === "takes")
      for (const take of part.takes) {
        const scene = sceneOfPart(take.scene);
        if (scene !== undefined)
          return {
            id: take.id,
            title: take.title,
            scene,
          };
      }
  }
  return undefined;
}

/** The card of an artifact that asks, at the foot of the thread, as the Band card draws it: its title and what it is for in the band, its first picture in the well, and Open, which its parts are answered from. */
export function ArtifactCard({
  machine,
  id,
  artifact,
}: {
  machine: Machine;
  id: string;
  artifact: JobArtifact;
}) {
  const picture = firstPicture(artifact);
  const open = () => go(artifactPath(machine.origin, id, artifact.id));
  const last = artifact.revisions[artifact.revisions.length - 1];
  const asking = asks(artifact);
  const facts: BandFacts = {
    form: artifact.form,
    title: artifact.title,
    stand: asking ? "asking" : artifact.state,
    words: artifact.words,
    ...(last === undefined
      ? {}
      : {
          revision: {
            number: last.number,
            said: revisionSaid(last.number, last.by, last.at),
          },
        }),
    ...(last?.by === "owner"
      ? {
          yours: `r${String(last.number)} is your edit`,
        }
      : {}),
    forLine: artifact.for_line,
  };
  return (
    <BandCard
      as="div"
      facts={facts}
      keep={`pocket:${machine.origin}:${id}:artifact:${artifact.id}:card`}
      {...(picture === undefined || !("scene" in picture)
        ? {}
        : {
            well: (
              <Well cut={false} sketch>
                <div
                  className="pk-bc__sketch"
                  style={{
                    aspectRatio: String(aspectOf(picture.scene)),
                  }}
                >
                  <SketchScene scene={picture.scene} title={picture.title} preview />
                </div>
              </Well>
            ),
          })}
      acts={{
        open,
      }}
      asked={asking}
    />
  );
}
