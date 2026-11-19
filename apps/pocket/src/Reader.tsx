// An artifact opened, as native Bridge's narrow reader draws it, "Fold" (`crates/bridge-native/src/job_view/reader_draw.rs`): the
// header band filled with where it stands, the title at text-xl, the revision picker and the contents in the band's strip, the
// answers in the band, and the words under it, each note under its paragraph. Tables and figures take the full width.

import type { ReactNode } from "react";
import { List } from "lucide-react";
import {
  Answers,
  AskBox,
  DismissAct,
  ForLineView,
  KindTile,
  RevisionChip,
  StateChip,
  YoursPill,
  useAsk,
  type BandActs,
  type BandFacts,
} from "./kit/components/compositions/BandCard/BandCard";
import {
  DropdownMenu,
  type DropdownMenuEntry,
} from "./kit/components/primitives/DropdownMenu/DropdownMenu";
import { Back } from "./ui";
import type { Head } from "./PageBody";

/** One revision a picker offers: its number, who kept it and when. */
export type RevisionOf = {
  number: number;
  by: "agent" | "owner";
  at: string;
};

const stamp = (at: string): string => {
  const when = new Date(at);
  return Number.isNaN(when.getTime())
    ? ""
    : when.toLocaleString("en-GB", {
        day: "numeric",
        month: "short",
        hour: "2-digit",
        minute: "2-digit",
        hour12: false,
      });
};

/** The revision picker: a menu of every revision, newest first, each with who kept it and when. */
function Picker({
  revisions,
  shown,
  onPick,
}: {
  revisions: readonly RevisionOf[];
  shown: number;
  onPick: (revision: number) => void;
}) {
  const entries = [
    ...revisions,
  ]
    .sort((a, b) => b.number - a.number)
    .map(
      (one): DropdownMenuEntry => ({
        kind: "item",
        id: String(one.number),
        label: `Revision ${String(one.number)}`,
        detail: `${one.by === "owner" ? "you" : "the Job"}, ${stamp(one.at)}`,
        selected: one.number === shown,
      }),
    );
  return (
    <DropdownMenu
      plain
      align="start"
      triggerLabel={`Revision ${String(shown)}`}
      triggerTooltip="Revision"
      entries={entries}
      onSelect={(id) => onPick(Number(id))}
    />
  );
}

/** The contents: a menu of the page's headings, which scrolls to the one chosen. */
function Contents({ heads }: { heads: readonly Head[] }) {
  const entries = heads.map(
    (one): DropdownMenuEntry => ({
      kind: "item",
      id: one.id,
      label: one.text,
      ...(one.level > 1
        ? {
            member: true,
          }
        : {}),
    }),
  );
  return (
    <DropdownMenu
      plain
      align="start"
      icon={List}
      triggerLabel="Contents"
      entries={entries}
      onSelect={(id) =>
        document.getElementById(id)?.scrollIntoView({
          block: "start",
        })
      }
    />
  );
}

export function Reader({
  facts,
  keep,
  onBack,
  back = "Back to the Job",
  revisions,
  shown,
  onPick,
  heads,
  acts,
  says,
  ready,
  children,
}: {
  facts: BandFacts;
  /** Names what he types in the note box, which is kept. */
  keep: string;
  onBack: () => void;
  back?: string;
  revisions?: readonly RevisionOf[];
  /** The revision the words are of, which is the newest unless a picker chose another. */
  shown?: number;
  onPick?: (revision: number) => void;
  heads?: readonly Head[];
  /** Its answers: each is absent where the reader cannot send it, and all are while an earlier revision is read. */
  acts?: BandActs;
  says?: string | undefined;
  /** It can be sent back with nothing typed, since notes beside the words say enough. */
  ready?: boolean;
  children: ReactNode;
}) {
  const newest = revisions?.reduce((most, one) => Math.max(most, one.number), 0);
  const earlier = shown !== undefined && newest !== undefined && shown < newest;
  const asking = facts.stand === "asking" && !earlier && acts !== undefined;
  const ask = useAsk(keep, asking);
  const noted = acts?.ask !== undefined;
  return (
    <section className="pk-screen pk-reader" aria-label={facts.title}>
      <header className="pk-reader__head" data-stand={facts.stand}>
        <div className="pk-reader__top">
          <KindTile form={facts.form} />
          <h1 className="pk-reader__title">{facts.title}</h1>
          <Back label={back} onClick={onBack} />
        </div>
        <div className="pk-reader__strip">
          <StateChip stand={facts.stand} words={facts.words} />
          {revisions === undefined || revisions.length === 0 || shown === undefined ? (
            facts.revision === undefined ? null : (
              <RevisionChip {...facts.revision} />
            )
          ) : (
            <Picker revisions={revisions} shown={shown} onPick={(revision) => onPick?.(revision)} />
          )}
          {facts.yours === undefined || earlier ? null : <YoursPill said={facts.yours} />}
          {heads === undefined || heads.length < 2 ? null : <Contents heads={heads} />}
        </div>
        <ForLineView line={facts.forLine} />
        {asking && acts !== undefined ? (
          <div className="pk-reader__acts">
            <Answers acts={acts} ask={ask} />
            {acts.dismiss === undefined ? null : <DismissAct onDismiss={acts.dismiss} />}
          </div>
        ) : null}
      </header>
      <div className="pk-body pk-reader__body">
        {earlier ? (
          <p className="pk-reader__earlier" role="status">
            Revision {shown} is for reading. Revision {newest} is the one you answer.
          </p>
        ) : null}
        {ask.noting && noted && acts?.ask !== undefined ? (
          <AskBox ask={ask} onSend={acts.ask} says={says} ready={ready === true} />
        ) : says === undefined ? null : (
          <p className="pk-bc__says" role="status">
            {says}
          </p>
        )}
        {children}
      </div>
    </section>
  );
}
