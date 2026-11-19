// An artifact as native Bridge draws it, in the looks the owner approved on 14 Nov: the kind's tile, the title, the revision and
// where it stands in a band filled with the state's colour, with what it is for under them. In the thread it is a card with its
// first screen in a well and its answers in a footer, or one line once it has a verdict. `crates/bridge-native/src/job_view/
// band_card.rs` is the card, `sketch_card.rs` the line.

import { useEffect, useState, type ReactNode } from "react";
import {
  BadgeCheck,
  Ban,
  CircleDot,
  ExternalLink,
  FileText,
  GitPullRequest,
  Layers,
  Pencil,
  Reply,
  ScrollText,
  type LucideIcon,
} from "lucide-react";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { forgetKept, readKept } from "../../keep";

export type Form = "sketch" | "draft" | "page";

/** Where an artifact stands, which colours its band and words its chip. */
export type Stand = "asking" | "approved" | "sent_back" | "reading" | "withdrawn" | "dismissed";

/** The word on a state's chip and the sentence its tooltip says, as native's `Stand` has them. */
export const STAND: Record<
  Stand,
  {
    word: string;
    tip: string;
    mark: LucideIcon;
  }
> = {
  asking: {
    word: "Asking",
    tip: "Waiting on your answer",
    mark: CircleDot,
  },
  approved: {
    word: "Approved",
    tip: "You approved it",
    mark: BadgeCheck,
  },
  sent_back: {
    word: "Sent back",
    tip: "You sent it back with changes",
    mark: Reply,
  },
  reading: {
    word: "Reading",
    tip: "Shown to you, with no answer asked",
    mark: CircleDot,
  },
  withdrawn: {
    word: "Withdrawn",
    tip: "The Job took it back",
    mark: Ban,
  },
  dismissed: {
    word: "Dismissed",
    tip: "You passed on it",
    mark: Ban,
  },
};

/** A kind's glyph and word: a sketch is a step's, a draft a setup's, a page a gate's. */
export const FORM: Record<
  Form,
  {
    glyph: LucideIcon;
    said: string;
  }
> = {
  sketch: {
    glyph: Layers,
    said: "Sketch",
  },
  draft: {
    glyph: FileText,
    said: "Draft",
  },
  page: {
    glyph: ScrollText,
    said: "Page",
  },
};

export const isSettled = (stand: Stand) =>
  stand === "approved" || stand === "sent_back" || stand === "withdrawn" || stand === "dismissed";

/** The 26 px tile that carries the kind's glyph. */
export function KindTile({ form }: { form: Form }) {
  const { glyph: Glyph, said } = FORM[form];
  return (
    <Tooltip label={said}>
      <span className="pk-bc__tile" data-form={form} role="img" aria-label={said}>
        <Glyph size={16} strokeWidth={2} aria-hidden />
      </span>
    </Tooltip>
  );
}

/** The state chip: a dot and its word, in the state's colour. `words`, where Fleet gave any, is its tooltip. */
export function StateChip({ stand, words }: { stand: Stand; words?: string | undefined }) {
  const { word, tip } = STAND[stand];
  return (
    <Tooltip label={words === undefined || words.trim() === "" ? tip : words}>
      <span className="pk-bc__chip" data-stand={stand} role="status" aria-label={word}>
        <CircleDot size={12} strokeWidth={2} aria-hidden />
        {word}
      </span>
    </Tooltip>
  );
}

/** The revision chip: `r3` in mono, who made it and when in its tooltip. */
export function RevisionChip({ number, said }: { number: number; said: string }) {
  return (
    <Tooltip label={said}>
      <span className="pk-bc__rev" aria-label={`Revision ${String(number)}`}>
        r{number}
      </span>
    </Tooltip>
  );
}

/** The pill that says the newest revision is his, in his colour. */
export function YoursPill({ said }: { said: string }) {
  return (
    <Tooltip label={said}>
      <span className="pk-bc__yours" aria-label="Your edit">
        <Pencil size={12} strokeWidth={2} aria-hidden />
        Your edit
      </span>
    </Tooltip>
  );
}

export type ForLine = {
  text?: string | undefined;
  links: readonly {
    kind: "plan_step" | "task" | "pull_request" | "link";
    label: string;
    target: string;
  }[];
};

const LINK_GLYPH = {
  pull_request: GitPullRequest,
  plan_step: CircleDot,
  task: CircleDot,
  link: ExternalLink,
} as const;

/** What it is for: "For:" and the sentence, then a chip for each link. */
export function ForLineView({ line }: { line: ForLine | undefined }) {
  const text = line?.text?.trim() ?? "";
  if (line === undefined || (text === "" && line.links.length === 0)) return null;
  return (
    <div className="pk-bc__for">
      <span className="pk-bc__for-head">For:</span>
      {text === "" ? null : <span className="pk-bc__for-text">{text}</span>}
      {line.links.map((link) => {
        const Glyph = LINK_GLYPH[link.kind];
        return (
          <span key={link.target} className="pk-bc__link">
            <Glyph size={12} strokeWidth={2} aria-hidden />
            <span>{link.label}</span>
          </span>
        );
      })}
    </div>
  );
}

/** What the artifact list says of a thread row, which the row alone does not: set by the host (`withFacts`). */
export type RowFacts = {
  /** The newest revision's number and what its tooltip says. */
  revision?: {
    number: number;
    said: string;
  };
  yours?: string;
  forLine?: ForLine;
  /** His words on how it ended. */
  words?: string;
};

/** Who kept a revision and when, for its chip's tooltip: `r3 by the Job, 19 Nov 14:05`. */
export function revisionSaid(number: number, by: "agent" | "owner", at: string): string {
  const who = by === "owner" ? "you" : "the Job";
  const when = new Date(at);
  const day = Number.isNaN(when.getTime())
    ? ""
    : `, ${when.toLocaleString("en-GB", {
        day: "numeric",
        month: "short",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      })}`;
  return `r${String(number)} by ${who}${day}`;
}

/** What the header draws. */
export type BandFacts = {
  form: Form;
  title: string;
  stand: Stand;
  /** His words on the verdict, the chip's tooltip. */
  words?: string | undefined;
  /** The newest revision's number and what its tooltip says. */
  revision?: {
    number: number;
    said: string;
  };
  /** The newest revision is his: what the pill's tooltip says. */
  yours?: string | undefined;
  forLine?: ForLine | undefined;
};

/** The header band: the kind's tile, the title, the revision and where it stands, and under them what it is for. */
export function BandHead({ facts, children }: { facts: BandFacts; children?: ReactNode }) {
  return (
    <div className="pk-bc__head" data-stand={facts.stand}>
      <div className="pk-bc__top">
        <KindTile form={facts.form} />
        <span className="pk-bc__title">{facts.title}</span>
        {facts.revision === undefined ? null : <RevisionChip {...facts.revision} />}
        {facts.yours === undefined ? null : <YoursPill said={facts.yours} />}
        <StateChip stand={facts.stand} words={facts.words} />
      </div>
      <ForLineView line={facts.forLine} />
      {children}
    </div>
  );
}

/** The first screen's well: sunken, a hairline edge, cut where it runs on, with a fade at its foot. */
export function Well({
  children,
  cut = true,
  sketch = false,
}: {
  children: ReactNode;
  cut?: boolean;
  sketch?: boolean;
}) {
  return (
    <div className="pk-bc__well-box">
      <div
        className={sketch ? "pk-bc__well pk-bc__well--sketch" : "pk-bc__well"}
        data-cut={cut || undefined}
      >
        {children}
      </div>
    </div>
  );
}

/** What a card's answers do. Each is absent where the card cannot send it. */
export type BandActs = {
  approve?: () => void;
  /** Sends the card back with this note. */
  ask?: (note: string) => void;
  dismiss?: () => void;
  open?: () => void;
};

/** Ask for changes: whether its note box is open, and what is typed in it, which `keep` holds. */
export function useAsk(keep: string, asking: boolean) {
  const noteKey = `${keep}:note`;
  const [noting, setNoting] = useState(() => readKept<string>(noteKey) !== undefined);
  const [note, setNote] = useState("");
  const cancel = () => {
    forgetKept(noteKey);
    setNote("");
    setNoting(false);
  };
  useEffect(() => {
    if (!asking) cancel();
  }, [
    asking,
    noteKey,
  ]);
  return {
    noteKey,
    noting: asking && noting,
    open: () => setNoting(true),
    note,
    setNote,
    cancel,
  };
}

/** What should change: the note, with Cancel and Send under it. */
export function AskBox({
  ask,
  onSend,
  says,
  ready = false,
}: {
  ask: ReturnType<typeof useAsk>;
  onSend: (note: string) => void;
  says?: string | undefined;
  /** It can be sent with nothing typed: notes beside it say enough. */
  ready?: boolean;
}) {
  return (
    <div className="pk-bc__ask-box" role="group" aria-label="Ask for changes">
      <label className="pk-bc__ask" htmlFor={ask.noteKey}>
        What should change?
      </label>
      <Textarea
        id={ask.noteKey}
        aria-label="What should change?"
        rows={3}
        autoFocus
        keep={ask.noteKey}
        value={ask.note}
        onChange={(event) => ask.setNote(event.currentTarget.value)}
      />
      <div className="pk-bc__acts" data-end>
        <Tooltip label="Keep it as it is">
          <button type="button" className="pk-bc__act" data-kind="ghost" onClick={ask.cancel}>
            Cancel
          </button>
        </Tooltip>
        <Tooltip label="Send it back with this note">
          <button
            type="button"
            className="pk-bc__act"
            data-kind="primary"
            disabled={ask.note.trim() === "" && !ready}
            onClick={() => {
              onSend(ask.note.trim());
              ask.cancel();
            }}
          >
            Send
          </button>
        </Tooltip>
      </div>
      {says === undefined ? null : (
        <p className="pk-bc__says" role="status">
          {says}
        </p>
      )}
    </div>
  );
}

/** Approve, Ask for changes and Dismiss: the answers a card and a reader's band both carry. */
export function Answers({ acts, ask }: { acts: BandActs; ask: ReturnType<typeof useAsk> }) {
  return (
    <>
      {acts.approve === undefined ? null : (
        <Tooltip label="Approve it as it stands">
          <button type="button" className="pk-bc__act" data-kind="approve" onClick={acts.approve}>
            Approve
          </button>
        </Tooltip>
      )}
      {acts.ask === undefined ? null : (
        <Tooltip label="Send it back with a note">
          <button type="button" className="pk-bc__act" data-kind="secondary" onClick={ask.open}>
            Ask for changes
          </button>
        </Tooltip>
      )}
    </>
  );
}

/** Dismiss, which puts it away with no answer. */
export function DismissAct({ onDismiss }: { onDismiss: () => void }) {
  return (
    <Tooltip label="Put it away with no answer">
      <button type="button" className="pk-bc__act" data-kind="ghost" onClick={onDismiss}>
        Dismiss
      </button>
    </Tooltip>
  );
}

/**
 * The footer: Approve and Ask for changes while it asks, Dismiss at the right, and Open. Ask for changes opens the note box in
 * their place, which keeps what is typed in it under `keep`.
 */
export function BandFoot({
  keep,
  asking,
  acts,
  says,
}: {
  keep: string;
  asking: boolean;
  acts: BandActs;
  /** A refusal's sentence, or what a send came to. */
  says?: string | undefined;
}) {
  const ask = useAsk(keep, asking);
  if (!asking && acts.open === undefined) return null;
  if (ask.noting && acts.ask !== undefined)
    return (
      <div className="pk-bc__foot">
        <AskBox ask={ask} onSend={acts.ask} says={says} />
      </div>
    );
  return (
    <div className="pk-bc__foot">
      <div className="pk-bc__acts">
        {asking ? <Answers acts={acts} ask={ask} /> : null}
        <span className="pk-bc__rest">
          {asking && acts.dismiss !== undefined ? <DismissAct onDismiss={acts.dismiss} /> : null}
          {acts.open === undefined ? null : (
            <Tooltip label="Read it on its own">
              <button type="button" className="pk-bc__act" data-kind="ghost" onClick={acts.open}>
                Open
              </button>
            </Tooltip>
          )}
        </span>
      </div>
      {says === undefined ? null : (
        <p className="pk-bc__says" role="status">
          {says}
        </p>
      )}
    </div>
  );
}

/** The start of his note on one line, ending in an ellipsis where it goes on. */
const NOTE_START = 80;
export function noteStart(words: string): string {
  const one = words.split(/\s+/).filter(Boolean).join(" ");
  return one.length > NOTE_START ? `${one.slice(0, NOTE_START).trimEnd()}…` : one;
}

/** A card with a verdict, in one line: the kind's glyph, the title, the verdict, and the start of his note. */
export function BandFold({
  form,
  title,
  stand,
  words,
  word: wordOver,
  said: tipOver,
  onOpen,
}: {
  form: Form;
  title: string;
  stand: Stand;
  words?: string | undefined;
  /** Another word for the verdict, and its tooltip: a card a later one replaced. */
  word?: string;
  said?: string;
  onOpen?: (() => void) | undefined;
}) {
  const { glyph: Glyph, said } = FORM[form];
  const { word: stood, tip: stoodTip, mark: Mark } = STAND[stand];
  const word = wordOver ?? stood;
  const tip = tipOver ?? stoodTip;
  const body = (
    <>
      <Glyph size={12} strokeWidth={2} aria-hidden />
      <span className="pk-bc__fold-title">{title}</span>
      <Tooltip label={tip}>
        <span className="pk-bc__verdict" data-stand={stand} role="status" aria-label={word}>
          <Mark size={12} strokeWidth={2} aria-hidden />
          {word}
        </span>
      </Tooltip>
      <span className="pk-bc__fold-words">
        {words === undefined || words.trim() === "" ? null : noteStart(words)}
      </span>
    </>
  );
  return onOpen === undefined ? (
    <div className="pk-bc__fold" data-stand={stand}>
      {body}
    </div>
  ) : (
    <button
      type="button"
      className="pk-bc__fold"
      data-stand={stand}
      aria-label={`Open ${said.toLowerCase()} ${title}`}
      onClick={onOpen}
    >
      {body}
    </button>
  );
}

/**
 * One artifact in the thread: the band card while it asks or is read, one line once it has a verdict. `well` is its first
 * screen, which a press on the band or the well opens.
 */
export function BandCard({
  facts,
  keep,
  well,
  between,
  acts,
  says,
  held = false,
  asked,
  as: Wrap = "li",
}: {
  facts: BandFacts;
  keep: string;
  well?: ReactNode;
  /** Under the screen and over the answers: a revision picker. */
  between?: ReactNode;
  acts: BandActs;
  says?: string | undefined;
  /** A card an answer was already sent from, which draws as it was. */
  held?: boolean;
  /** Whether its answers show: it asks, and can be answered. Where absent, the state says. */
  asked?: boolean;
  /** The element it is: an item of a list, or a block of its own. */
  as?: "li" | "div";
}) {
  if (isSettled(facts.stand) && !held)
    return (
      <Wrap className="pk-bc-row">
        <BandFold
          form={facts.form}
          title={facts.title}
          stand={facts.stand}
          words={facts.words}
          onOpen={acts.open}
        />
      </Wrap>
    );
  const asking = asked ?? facts.stand === "asking";
  return (
    <Wrap className="pk-bc-row">
      <article
        className="pk-bc"
        role="article"
        aria-label={`${FORM[facts.form].said}: ${facts.title}${facts.revision === undefined ? "" : `, Revision ${String(facts.revision.number)}`}`}
      >
        {acts.open === undefined ? (
          <>
            <BandHead facts={facts} />
            {well}
          </>
        ) : (
          <button
            type="button"
            className="pk-bc__press"
            aria-label={`Open ${FORM[facts.form].said.toLowerCase()} ${facts.title}`}
            onClick={acts.open}
          >
            <BandHead facts={facts} />
            {well}
          </button>
        )}
        {between === undefined || between === null ? null : (
          <div className="pk-bc__between">{between}</div>
        )}
        <BandFoot keep={keep} asking={asking} acts={acts} says={says} />
      </article>
    </Wrap>
  );
}
