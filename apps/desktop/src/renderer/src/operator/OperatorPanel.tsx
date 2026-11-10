// The Operator's page: its conversation over the Job composer, full width, and nothing else. What it offers is a Job, which it
// draws as a card with Dispatch and Leave it; the Job it started is drawn as a Job's thread draws one. A Job opens over it
// (`overlay.tsx`). Drawn only where the window has an Operator store (`store.tsx`).

import { Fragment, useState } from "react";
import { FileSearch } from "lucide-react";
import {
  Button,
  JobCard,
  DropdownMenu,
  SessionComposer,
  SessionThread,
  SpaciousTake,
  Switch,
  type ComposerMode,
  type ComposerTag,
  type SessionThreadRow,
  type ThreadCard,
} from "@armada/components";
import type { DrawnSketch, Session } from "@armada/screens/src/draft/sessions";

import { searchFiles } from "../commands";
import { useNumbered } from "../numbered";
import { hostedCommands } from "../session-runner";
import { ChildJob } from "../session-view/ChildJob";
import { SketchSheet } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { offerTitle } from "./model";
import type { OperatorMessage } from "./model";
import { JobOver, openJobOver } from "./overlay";
import { useOperator } from "./store";
import type { OperatorStore } from "./store";
import "./operator.css";

export function OperatorPanel() {
  const store = useOperator();
  if (store === null) return null;
  return (
    <div className="armada-operator">
      <Talk store={store} />
      <JobOver />
    </div>
  );
}

/** Opens a Job from a card: over the Operator, which stays as it was behind it. */
const openJob = openJobOver;

const NONE: readonly never[] = [];
const NO_SKETCHES: DrawnSketch[] = [];
/** Where a tag stands in what the composer sends. */
const CHIP = "￼";

/** The composer's words with each tag written in by its title, as the conversation shows them. */
const spelled = (text: string, tags: readonly ComposerTag[]): string => {
  let at = 0;
  return text.replaceAll(CHIP, () => `@${tags[at++]?.title ?? ""}`).trim();
};

/** A Job the Operator offered to read the code: its brief, and Dispatch into a repository, or Leave it. */
function OfferedJob({
  offer,
  onSettle,
}: {
  offer: NonNullable<OperatorMessage["offer"]>;
  onSettle: (repository: string | null) => void;
}) {
  const title = offerTitle(offer.brief);
  const only = offer.in.length === 1 ? offer.in[0] : undefined;
  return (
    <span className="armada-operator__offer" role="group" aria-label={`Job offered: ${title}`}>
      <span className="armada-operator__offer-kind">
        <FileSearch size={12} strokeWidth={2} aria-hidden />
        Job offered
      </span>
      <span className="armada-operator__offer-brief">{offer.brief}</span>
      {offer.left === true ? (
        <span className="armada-operator__does">Left</span>
      ) : (
        <span className="armada-operator__offer-acts">
          {offer.in.length === 0 ? (
            <span className="armada-operator__does">
              No repository is served here to start it in
            </span>
          ) : only !== undefined ? (
            <Button
              size="sm"
              variant="secondary"
              aria-label={`Dispatch in ${only}: ${title}`}
              onClick={() => onSettle(only)}
            >
              Dispatch
            </Button>
          ) : (
            <DropdownMenu
              triggerLabel="Dispatch"
              triggerAriaLabel={`Dispatch: ${title}`}
              align="start"
              portal
              entries={offer.in.map((one) => ({
                kind: "item" as const,
                id: one,
                label: `In ${one}`,
              }))}
              onSelect={(one) => onSettle(one)}
            />
          )}
          <Button size="sm" variant="secondary" onClick={() => onSettle(null)}>
            Leave it
          </Button>
        </span>
      )}
    </span>
  );
}

/** When a message the mock wrote without a time was said: in the order it came, from when the window opened. */
const OPENED = Date.now();
const atOf = (message: OperatorMessage, index: number) =>
  message.at ?? new Date(OPENED + index * 1000).toISOString();

/** Discussion's words as a Job thread's rows, so they read as a Job's thread does. */
function threadRows(store: OperatorStore): SessionThreadRow[] {
  return store.state.messages.flatMap((message, index): SessionThreadRow[] =>
    message.opens !== undefined || message.text === ""
      ? []
      : [
          {
            id: message.id,
            at: atOf(message, index),
            kind: "message",
            from: message.from === "you" ? "you" : "agent",
            text: message.text,
          },
        ],
  );
}

/** What Discussion draws between its words: a Job it started, or one it offered. */
function threadCards(store: OperatorStore, sessions: readonly Session[]): ThreadCard[] {
  const { act } = store;
  return store.state.messages.flatMap((message, index): ThreadCard[] => {
    const at = atOf(message, index);
    const cards: ThreadCard[] = [];
    if (message.opens !== undefined) {
      const opens = message.opens;
      const child = sessions.find((one) => one.id === opens.session);
      // The Job it started as a Job's thread draws one it started: the shared card, live from the Job's record once it is read.
      cards.push({
        key: `${message.id}:opens`,
        at,
        node:
          child === undefined ? (
            <JobCard
              title={opens.title}
              state="working"
              said="Starting"
              onOpen={() => openJob(opens.session)}
            />
          ) : (
            <ChildJob child={child} manifestId={child.manifestId} onOpen={openJob} />
          ),
      });
    }
    if (message.offer !== undefined && message.opens === undefined)
      cards.push({
        key: `${message.id}:offer`,
        at,
        node: (
          <OfferedJob
            offer={message.offer}
            onSettle={(repository) =>
              act({
                kind: "offer",
                message: message.id,
                repository,
              })
            }
          />
        ),
      });
    return cards;
  });
}

/** Talk: the conversation over the Job composer, as a Job's thread stands over its box. `@` Jobs and files, `#` issues and pull requests, `/` commands, files and sketches. */
function Talk({ store }: { store: OperatorStore }) {
  const { state, act } = store;
  const draft = useSessionsDraft();
  const sessions = useSessions();
  const numbered = useNumbered(undefined);
  const [tuned, setTuned] = useState<{
    model: string | null;
    effort: string | null;
  }>({
    model: null,
    effort: null,
  });
  const [mode, setMode] = useState<ComposerMode>("auto");
  const [drawn, setDrawn] = useState<DrawnSketch[]>(NO_SKETCHES);
  const [padOpen, setPadOpen] = useState(false);
  const messages = state.messages;
  const off = !state.on;
  const rows = threadRows(store);
  const first = rows[0]?.at;
  const cards = threadCards(store, sessions);
  const leading = cards.filter((one) => first === undefined || (one.at ?? "") < first);
  const after = cards.filter((one) => !leading.includes(one));
  return (
    <SpaciousTake.Provider value>
      <div className="armada-operator__talk-pane armada-session-frame" data-off={off || undefined}>
        {off ? (
          <div className="armada-operator__off">
            <Switch
              size="sm"
              checked={false}
              description="On, it answers here"
              onChange={() =>
                act({
                  kind: "on",
                  on: true,
                })
              }
            >
              Operator
            </Switch>
          </div>
        ) : null}
        <div className="armada-operator__conversation">
          {messages.length === 0 ? null : (
            <span className="armada-operator__clear">
              <Button
                size="sm"
                variant="ghost"
                onClick={() =>
                  act({
                    kind: "clear",
                  })
                }
              >
                Clear
              </Button>
            </span>
          )}
          <SessionThread
            sessionId="operator"
            agentName="Operator"
            rows={rows}
            // A card said before any words leads the thread: the thread places the rest after the words before them.
            {...(leading.length === 0
              ? {}
              : {
                  lead: leading.map((one) => <Fragment key={one.key}>{one.node}</Fragment>),
                })}
            inline={after}
            working={state.thinking !== undefined}
            onAnswer={() => undefined}
            onOpenSession={openJob}
          />
        </div>
        <SessionComposer
          label="Message to the Operator"
          disabled={off}
          working={false}
          mode={mode}
          onMode={setMode}
          model={tuned.model}
          effort={tuned.effort}
          models={draft?.models ?? NONE}
          {...(draft?.defaultModel === undefined
            ? {}
            : {
                defaultModel: draft.defaultModel,
              })}
          efforts={draft?.efforts ?? NONE}
          onTune={setTuned}
          commands={hostedCommands(draft?.commands ?? NONE)}
          taggable={sessions
            .filter((one) => one.dead === undefined && one.title !== undefined)
            .map(
              (one): ComposerTag => ({
                kind: "session",
                id: one.id,
                title: one.title!,
              }),
            )}
          searchFiles={(query) => searchFiles(query)}
          {...(numbered === undefined
            ? {}
            : {
                numbered,
              })}
          tags={NONE}
          onTags={() => undefined}
          draftKey="operator:box"
          drawn={drawn.map(({ id, title }) => ({
            id,
            title,
          }))}
          onDraw={() => setPadOpen(true)}
          onRemoveDrawn={(id) => setDrawn((was) => was.filter((one) => one.id !== id))}
          grows="panel"
          onSend={(sent) => {
            const words = spelled(sent.text, sent.tags);
            const attached = [
              ...sent.files.map((one) => one.name),
              ...drawn.map((one) => one.title),
            ];
            if (words === "" && attached.length === 0) return Promise.resolve(false);
            act({
              kind: "say",
              text: words,
              attached,
            });
            setDrawn(NO_SKETCHES);
            return Promise.resolve(true);
          }}
        />
        <SketchSheet
          open={padOpen}
          onClose={() => setPadOpen(false)}
          onAttach={(sketch) => {
            setDrawn((was) => [
              ...was,
              sketch,
            ]);
            setPadOpen(false);
          }}
        />
      </div>
    </SpaciousTake.Provider>
  );
}
