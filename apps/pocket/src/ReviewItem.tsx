// One item of To review, drawn in the form it asks in and answered in place, with no Job to open. The frame is the Band
// card's: the kind's tile, what it asks and where it stands, then the Job it comes from and why it stands there. Under it
// is the form: an artifact's parts, a plan, a draft, a Ready to try card, options, a to do, a command or a press.
//
// **A new form is one case of `Form`**, read off the item's `kind` and `answer`, so a field another form needs slots in
// there and nowhere else. An artifact's parts answer as the Job's own screen answers them (`ArtifactParts`), since a pick
// of takes has no form in `answer_items`; every other Approve goes through `answer_items`.

import { useState } from "react";
import {
  AppWindow,
  CircleCheck,
  CircleHelp,
  FileText,
  Flag,
  GitMerge,
  GitPullRequest,
  Layers,
  ListChecks,
  Play,
  RefreshCw,
  Rocket,
  ScrollText,
  SquareCheck,
  Sparkles,
  SquareTerminal,
  type LucideIcon,
} from "lucide-react";
import { Button } from "./kit/components/primitives/Button/Button";
import { Textarea } from "./kit/components/primitives/Textarea/Textarea";
import { Tooltip } from "./kit/components/primitives/Tooltip/Tooltip";
import {
  AskBox,
  BandHead,
  revisionSaid,
  useAsk,
  Well,
  type BandFacts,
} from "./kit/components/compositions/BandCard/BandCard";
import { DraftText } from "./kit/components/compositions/SessionThread/DraftRow";
import { PlanCard } from "./kit/components/compositions/SessionThread/SessionThread";
import type { ToReviewItem, ToReviewKind } from "./wire";

import { ArtifactParts, PictureBox } from "./Artifact";
import { WithKeptStyles } from "./keptStyles";
import { useArtifacts, partsOf } from "./artifacts";
import { OnMac, PressCard, WaitingAsk } from "./cards";
import type { Mac } from "./fleet";
import { actsOn, attempt, useJob } from "./jobState";
import { TryCard, useTryCards } from "./ReadyToTry";
import { refreshPreview } from "./refresh";
import { artifactPath, documentPath, go, jobPath, sketchPath } from "./router";
import { approvalOf, draftsOf, sessionOfThread, sketchOf } from "./thread";
import { answersOn, readList } from "./reviewList";
import { Trouble } from "./ui";

/** Each kind's glyph and the word its tooltip names it by. */
const KINDS: Record<
  ToReviewKind,
  {
    glyph: LucideIcon;
    said: string;
  }
> = {
  plan: {
    glyph: ListChecks,
    said: "Plan",
  },
  sketch: {
    glyph: Layers,
    said: "Sketch",
  },
  document: {
    glyph: ScrollText,
    said: "Page",
  },
  draft: {
    glyph: FileText,
    said: "Draft",
  },
  ready_to_try: {
    glyph: Rocket,
    said: "Ready to try",
  },
  clash: {
    glyph: GitMerge,
    said: "Clash",
  },
  goal_met: {
    glyph: Flag,
    said: "Goal met",
  },
  question: {
    glyph: CircleHelp,
    said: "Question",
  },
  to_do: {
    glyph: SquareCheck,
    said: "To do",
  },
  walk: {
    glyph: AppWindow,
    said: "Walk",
  },
  command: {
    glyph: SquareTerminal,
    said: "Command",
  },
  pull_request: {
    glyph: GitPullRequest,
    said: "Pull request",
  },
  restart: {
    glyph: RefreshCw,
    said: "Restart",
  },
  done: {
    glyph: CircleCheck,
    said: "Done",
  },
};

/** The frame's head where the form draws none: the kind's tile and what the item asks. */
function FrameHead({ item }: { item: ToReviewItem }) {
  const { glyph: Glyph, said } = KINDS[item.kind];
  return (
    <div className="pk-bc__head" data-stand="asking">
      <div className="pk-bc__top">
        <Tooltip label={item.row.glyph.said === item.row.text ? said : item.row.glyph.said}>
          <span
            className="pk-bc__tile pk-rv__tile"
            data-kind={item.kind}
            role="img"
            aria-label={said}
          >
            <Glyph size={16} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
        <span className="pk-bc__title">{item.row.text}</span>
      </div>
    </div>
  );
}

type Said = (says: string) => void;

/** What every form is handed: the item, its Mac, a key for what is typed, and where a refusal's sentence goes. */
type FormProps = {
  mac: Mac;
  item: ToReviewItem;
  keep: string;
  onSaid: Said;
};

/** Runs an answer, says its refusal, and answers whether it went. */
const tried = async (act: () => Promise<void>, mac: Mac, onSaid: Said): Promise<boolean> => {
  onSaid("");
  const refused = await attempt(act, mac.machine);
  if (refused !== null) onSaid(refused);
  return refused === null;
};

/** Approve and Ask for changes, sent through `answer_items` for the item's Job. */
function Verdict({
  mac,
  item,
  keep,
  onSaid,
  open,
}: FormProps & {
  open?: () => void;
}) {
  const ask = useAsk(keep, true);
  const answers = answersOn(mac.machine);
  const send = (how: "approve" | "send_back", note?: string) =>
    void tried(
      () =>
        answers.answerItems({
          items: [
            {
              session_id: item.job.id,
              item: item.row.id,
              how,
              ...(note === undefined || note === ""
                ? {}
                : {
                    notes: [
                      {
                        text: note,
                      },
                    ],
                  }),
            },
          ],
        }),
      mac,
      onSaid,
    );
  if (ask.noting) return <AskBox ask={ask} onSend={(note) => send("send_back", note)} />;
  return (
    <div className="pk-bc__acts">
      <Tooltip label="Approve it as it stands">
        <button
          type="button"
          className="pk-bc__act"
          data-kind="approve"
          onClick={() => send("approve")}
        >
          Approve
        </button>
      </Tooltip>
      <Tooltip label="Send it back with a note">
        <button type="button" className="pk-bc__act" data-kind="secondary" onClick={ask.open}>
          Ask for changes
        </button>
      </Tooltip>
      {open === undefined ? null : (
        <span className="pk-bc__rest">
          <OpenAct open={open} />
        </span>
      )}
    </div>
  );
}

/** Open, which reads it on its own screen. */
function OpenAct({ open }: { open: () => void }) {
  return (
    <Tooltip label="Read it on its own">
      <button type="button" className="pk-bc__act" data-kind="ghost" onClick={open}>
        Open
      </button>
    </Tooltip>
  );
}

/** A sketch, a page or a draft of parts: its band, then every part, each answered on its own. */
function ArtifactForm(props: FormProps) {
  const { mac, item, keep } = props;
  const [read, reread] = useArtifacts(mac.machine, item.job.id);
  const artifact = read.artifacts?.find((one) => one.item === item.row.id);
  if (read.artifacts === undefined)
    return <div className="pk-rv__wait pk-skeleton" aria-hidden="true" />;
  if (artifact === undefined || partsOf(artifact).length === 0) {
    const act = item.row.act;
    const open =
      act.kind === "open_sketch"
        ? sketchPath(mac.machine.origin, item.job.id, act.title)
        : act.kind === "open_document"
          ? documentPath(mac.machine.origin, item.job.id, act.path)
          : undefined;
    return (
      <>
        <FrameHead item={item} />
        {item.kind === "sketch" ? <LoneSketch {...props} /> : null}
        {item.kind === "draft" ? (
          <DraftForm {...props} />
        ) : (
          <Verdict
            {...props}
            {...(open === undefined
              ? {}
              : {
                  open: () => go(open),
                })}
          />
        )}
      </>
    );
  }
  const last = artifact.revisions[artifact.revisions.length - 1];
  const facts: BandFacts = {
    form: artifact.form,
    title: artifact.title,
    stand: "asking",
    ...(last === undefined
      ? {}
      : {
          revision: {
            number: last.number,
            said: revisionSaid(last.number, last.by, last.at),
          },
        }),
    forLine: artifact.for_line,
  };
  return (
    <>
      <BandHead facts={facts} />
      <div className="pk-rv__parts">
        <ArtifactParts
          machine={mac.machine}
          id={item.job.id}
          artifact={artifact}
          keep={keep}
          reread={reread}
        />
      </div>
      <div className="pk-bc__acts" data-end>
        <OpenAct open={() => go(artifactPath(mac.machine.origin, item.job.id, artifact.id))} />
      </div>
    </>
  );
}

/** A sketch of no parts is the drawing its thread holds, which a press opens on its own. */
function LoneSketch({ mac, item }: FormProps) {
  const [read] = useJob(mac.machine, item.job.id);
  const session = read.thread === undefined ? undefined : sessionOfThread(read.thread);
  const drawn = session === undefined ? undefined : sketchOf(session, item.row.id);
  if (drawn === undefined) return null;
  return (
    <WithKeptStyles machine={mac.machine}>
      <PictureBox
        picture={{
          id: item.row.id,
          title: drawn.title,
          scene: drawn.scene,
        }}
        onOpen={() => go(sketchPath(mac.machine.origin, item.job.id, item.row.id))}
      />
    </WithKeptStyles>
  );
}

/** A draft with no parts is the text the thread's row proposed. */
function DraftForm(props: FormProps) {
  const { mac, item } = props;
  const [read] = useJob(mac.machine, item.job.id);
  const session = read.thread === undefined ? undefined : sessionOfThread(read.thread);
  const draft =
    session === undefined
      ? undefined
      : draftsOf(session).find((one) => `draft:${one.id}` === item.row.id);
  return (
    <>
      {draft === undefined ? null : (
        <Well>
          <DraftText text={draft.text} />
        </Well>
      )}
      <Verdict {...props} />
    </>
  );
}

/** A plan, drawn as the thread draws it, approved or sent back with a note. */
function PlanForm(props: FormProps) {
  const { mac, item, onSaid } = props;
  const [read] = useJob(mac.machine, item.job.id);
  const session = read.thread === undefined ? undefined : sessionOfThread(read.thread);
  const approval =
    session === undefined
      ? undefined
      : approvalOf(
          session,
          () =>
            void tried(
              () =>
                answersOn(mac.machine).answerItems({
                  items: [
                    {
                      session_id: item.job.id,
                      item: item.row.id,
                      how: "approve",
                    },
                  ],
                }),
              mac,
              onSaid,
            ),
        );
  if (approval === undefined) return <Verdict {...props} />;
  return (
    <>
      <PlanCard
        approval={{
          ...approval,
          taskText: true,
        }}
      />
      <Verdict {...props} />
    </>
  );
}

/** Each open Ready to try card the item holds, as the Job's own card draws and answers it. */
function TryForm({ mac, item }: FormProps) {
  const [cards, reread] = useTryCards(mac.machine, item.job.id, true);
  const wanted = item.answer.op === "answer_ready_to_try" ? item.answer.cards : [];
  const held = (cards ?? []).filter((card) =>
    wanted.some((one) => one.branch === card.branch && one.tip === card.tip),
  );
  if (cards === undefined) return <div className="pk-rv__wait pk-skeleton" aria-hidden="true" />;
  return (
    <>
      {held.map((card) => (
        <TryCard key={card.id} machine={mac.machine} id={item.job.id} card={card} reread={reread} />
      ))}
    </>
  );
}

/** A choice, his words, or both, through `answer_waiting`. */
function AskForm({ mac, item, keep, onSaid }: FormProps) {
  const act = item.row.act;
  const choices =
    act.kind === "answer"
      ? act.options.map((option, choice) => ({
          label: option.label,
          ...(option.description === undefined
            ? {}
            : {
                description: option.description,
              }),
          choice,
        }))
      : [];
  return (
    <WaitingAsk
      text={item.row.text}
      choices={choices}
      words={act.kind === "answer" ? act.other : true}
      keep={keep}
      headed
      answer={(given) =>
        tried(
          () =>
            answersOn(mac.machine).answerWaiting({
              session_id: item.job.id,
              item_id: item.row.id,
              ...given,
            }),
          mac,
          onSaid,
        )
      }
    />
  );
}

/** Something he does on his side, then marks done with a note if he has one. */
function DoneForm({ mac, item, keep, onSaid }: FormProps) {
  const [note, setNote] = useState("");
  const steps =
    item.row.act.kind === "done"
      ? (item.row.act.steps ?? [])
      : item.row.act.kind === "steps"
        ? item.row.act.steps
        : [];
  return (
    <div className="pk-ask" role="group" aria-label={item.row.text}>
      {steps.length === 0 ? null : (
        <ol className="pk-rv__steps">
          {steps.map((step) => (
            <li key={step}>{step}</li>
          ))}
        </ol>
      )}
      <Textarea
        keep={`${keep}:note`}
        rows={1}
        aria-label="Note"
        placeholder="Note"
        value={note}
        onChange={(event) => setNote(event.target.value)}
      />
      <div className="pk-bc__acts">
        <Tooltip label="Tell the Job it is done">
          <button
            type="button"
            className="pk-bc__act"
            data-kind="approve"
            onClick={() =>
              void tried(
                () =>
                  answersOn(mac.machine).doneWaiting({
                    session_id: item.job.id,
                    item_id: item.row.id,
                    ...(note.trim() === ""
                      ? {}
                      : {
                          note: note.trim(),
                        }),
                  }),
                mac,
                onSaid,
              ).then((went) => went && setNote(""))
            }
          >
            Done
          </button>
        </Tooltip>
      </div>
    </div>
  );
}

/** A Job held on its start card: Start begins it. */
function BeginForm({ mac, item, onSaid }: FormProps) {
  const session = item.answer.op === "begin_session" ? item.answer.session_id : item.job.id;
  return (
    <div className="pk-bc__acts">
      <Tooltip label="Start the Job as it was held">
        <button
          type="button"
          className="pk-bc__act"
          data-kind="approve"
          onClick={() =>
            void tried(
              () =>
                actsOn(mac.machine, () => void readList(mac.machine)).begin({
                  session_id: session,
                }),
              mac,
              onSaid,
            )
          }
        >
          <Play size={14} aria-hidden="true" />
          Start
        </button>
      </Tooltip>
      <span className="pk-bc__rest">
        <Tooltip label="Open its start card">
          <button
            type="button"
            className="pk-bc__act"
            data-kind="ghost"
            onClick={() => go(jobPath(mac.machine.origin, session))}
          >
            Open
          </button>
        </Tooltip>
      </span>
    </div>
  );
}

/** A command to run on the Mac from here. One meant for another Mac is said, and run there. */
function RunForm({
  mac,
  item,
  onSaid,
  command,
}: Omit<FormProps, "keep"> & {
  command: string;
}) {
  const [going, setGoing] = useState(false);
  return (
    <>
      <code className="pk-card__command">{command}</code>
      <div className="pk-bc__acts">
        {item.row.on === undefined ? (
          <Tooltip label="Run it on the Mac">
            <button
              type="button"
              className="pk-bc__act"
              data-kind="approve"
              disabled={going}
              onClick={() => {
                setGoing(true);
                void tried(
                  () =>
                    actsOn(mac.machine, () => void readList(mac.machine)).run({
                      session_id: item.job.id,
                      command,
                    }),
                  mac,
                  onSaid,
                ).finally(() => setGoing(false));
              }}
            >
              <Play size={14} aria-hidden="true" />
              Run
            </button>
          </Tooltip>
        ) : (
          <span className="pk-rv__on">Runs on {item.row.on}</span>
        )}
      </div>
    </>
  );
}

/** No operation answers it: the row's act is the press. */
function PressForm({ mac, item, onSaid }: FormProps) {
  const act = item.row.act;
  switch (act.kind) {
    case "run":
      return (
        <RunForm
          {...{
            mac,
            item,
            onSaid,
          }}
          command={act.command}
        />
      );
    case "steps":
      return (
        <>
          <ol className="pk-rv__steps">
            {act.steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
          {act.command === undefined ? null : (
            <code className="pk-card__command">{act.command}</code>
          )}
        </>
      );
    case "restart":
      return (
        <div className="pk-choices" role="group" aria-label="Options">
          <Button
            variant="primary"
            onClick={() => void tried(() => refreshPreview(mac.machine), mac, onSaid)}
          >
            <RefreshCw size={14} aria-hidden="true" />
            {act.label}
          </Button>
        </div>
      );
    case "open_job":
      return (
        <PressCard text={item.row.text} open={() => go(jobPath(mac.machine.origin, act.id))} />
      );
    case "approve_pr":
      return <OnMac text={`Pull request ${act.target}`} />;
    default:
      return <OnMac text={item.row.text} />;
  }
}

/** The form an item asks in, by its kind and the operation that answers it. */
function Form(props: FormProps) {
  const { item } = props;
  switch (item.answer.op) {
    case "answer_items":
      switch (item.kind) {
        case "plan":
          return <PlanForm {...props} />;
        case "sketch":
        case "document":
        case "draft":
          return <ArtifactForm {...props} />;
        case "walk":
          return (
            <>
              <OnMac text={item.row.text} />
              <Verdict {...props} />
            </>
          );
        default:
          return <Verdict {...props} />;
      }
    case "answer_waiting":
      return <AskForm {...props} />;
    case "answer_ready_to_try":
      return <TryForm {...props} />;
    case "done_waiting":
      return <DoneForm {...props} />;
    case "begin_session":
      return <BeginForm {...props} />;
    case "dismiss_waiting":
      return null;
    case "press":
      return <PressForm {...props} />;
  }
}

/** The forms whose own head names what they ask, so the frame draws no title over them. */
const HEADED = (item: ToReviewItem) =>
  item.answer.op === "answer_items" &&
  (item.kind === "sketch" || item.kind === "document" || item.kind === "draft");

/** Dismiss, where Fleet takes one for the row. */
function DismissAct({ mac, item, onSaid }: FormProps) {
  if (item.row.dismiss === "none") return null;
  return (
    <Tooltip
      label={
        item.row.dismiss === "skip"
          ? "Skip it; the Job is told nothing"
          : "Put it away with no answer"
      }
    >
      <button
        type="button"
        className="pk-bc__act"
        data-kind="ghost"
        onClick={() =>
          void tried(
            () =>
              answersOn(mac.machine).dismissWaiting({
                session_id: item.job.id,
                item_id: item.row.id,
                ...(item.row.dismiss === "not_a_question"
                  ? {
                      misread: true,
                    }
                  : {}),
              }),
            mac,
            onSaid,
          )
        }
      >
        {item.row.dismiss === "skip" ? "Skip" : "Dismiss"}
      </button>
    </Tooltip>
  );
}

/** What the Operator read in the item and the option it would pick, as native's review sheet draws it. */
function OperatorNote({ read, pick }: { read: string; pick: string }) {
  return (
    <div className="pk-rv__operator" role="group" aria-label="Operator">
      <span className="pk-rv__operator-head">
        <Sparkles size={14} aria-hidden="true" />
        Operator
      </span>
      <p>{read}</p>
      {pick.trim() === "" ? null : <p>{`Would pick: ${pick}`}</p>}
    </div>
  );
}

export function ReviewItem({
  mac,
  item,
  several,
}: {
  mac: Mac;
  item: ToReviewItem;
  several: boolean;
}) {
  const [says, setSays] = useState("");
  const keep = `pocket:${mac.machine.origin}:${item.job.id}:review:${item.row.id}`;
  const props: FormProps = {
    mac,
    item,
    keep,
    onSaid: setSays,
  };
  const macName = mac.machine.name || new URL(mac.machine.origin).hostname;
  return (
    <li className="pk-rv-row">
      <article className="pk-bc pk-rv" data-kind={item.kind} aria-label={item.row.text}>
        {HEADED(item) ? null : <FrameHead item={item} />}
        {item.why === undefined ? null : <p className="pk-rv__why">{item.why}</p>}
        {item.operator_read === undefined ? null : <OperatorNote {...item.operator_read} />}
        <div className="pk-rv__body">
          <Form {...props} />
        </div>
        <div className="pk-rv__from">
          <Tooltip label="Open the Job">
            <button
              type="button"
              className="pk-rv__job"
              onClick={() => go(jobPath(mac.machine.origin, item.job.id))}
            >
              {item.job.title}
            </button>
          </Tooltip>
          {several ? <span className="pk-rv__mac">{macName}</span> : null}
          <span className="pk-bc__rest">
            <DismissAct {...props} />
          </span>
        </div>
        <Trouble says={says} />
      </article>
    </li>
  );
}
