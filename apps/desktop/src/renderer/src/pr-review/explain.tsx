// The explanations: tiles in Flight control, each shown under the description in the stage's own overview. **Each is its own question** (how it
// works, what it changes for him as a user, how to test it, why it was needed), and an agent answers one only when he
// first chooses it. The answer is kept per commit, as a tool's result is, so choosing another and coming back shows it
// at once. Explanations are defined in the same three places the tools are, so a repository can add its own.

import { Bot, BookOpen, Check, CircleX, ExternalLink, Eye, FileText, History, LoaderCircle, RefreshCw, RotateCw, X } from "lucide-react";
import { Button, Kbd, Prose, PrMark, SessionComposer, Tooltip, agoOf } from "@armada/components";

import type { useSendFindings } from "./acts";
import { RunFailed, SOURCE_MARK } from "./marks";
import type { Account, Asked, ReviewPr, ToolDef } from "./model";
import { REVIEW_KEYS, capOf } from "./key-table";
import { NO_WORK, useReviewStore, useReviews, type ReviewStore } from "./store";
import { ThemeBlock } from "./themes";
import { onlyExplanations, toolsOf } from "./tools";

type Send = ReturnType<typeof useSendFindings>;

/**
 * What he asked of this pull request, above the description. **A question he sent from the composer at the foot of the reader is
 * kept here**, an index of what he asked, and nothing is drawn where he asked none. The explanations are Flight control's tiles.
 */
export function AskedOf({ pr }: { pr: ReviewPr }) {
  const work = useReviews().work[pr.key] ?? NO_WORK;
  if (work.asks.length === 0) return null;
  return (
    <section className="pr-explainer" aria-label="Questions asked">
      <h3 className="pr-what__title pr-explainer__title">Asked</h3>
      <Asks pr={pr} />
    </section>
  );
}

/**
 * Chooses an explanation, or a question he asked, by its id. **A choice with no kept result asks first**, since it spends
 * tokens, and one that is kept opens at once.
 */
export function chooseQuick(store: ReviewStore, pr: ReviewPr, id: string): void {
  const snap = store.get();
  const kinds = onlyExplanations(toolsOf(snap, pr.manifestId));
  const def = kinds.find((one) => one.id === id);
  if (def === undefined) return store.show(pr.key, id);
  const kept = (snap.work[pr.key] ?? NO_WORK).explains[def.id];
  if (kept !== undefined && kept.state !== "idle") store.explain(pr.key, def);
  else store.spend(def.name, () => store.explain(pr.key, def));
}

/** An explanation's tile in Flight control: the overview shows it, run first where nothing is kept for it. */
export function pressExplanation(store: ReviewStore, pr: ReviewPr, def: ToolDef): void {
  store.setView(pr.key, "explanation");
  chooseQuick(store, pr, def.id);
}

/** What was chosen: the explanation, or the answer to a question of his. Nothing where nothing is chosen. */
export function Explained({ pr, send, onOpenLink }: { pr: ReviewPr; send: Send; onOpenLink: (address: string) => void }) {
  const reviews = useReviews();
  const work = reviews.work[pr.key] ?? NO_WORK;
  const chosen = onlyExplanations(toolsOf(reviews, pr.manifestId)).find((one) => one.id === work.kind);
  const asked = work.asks.find((one) => one.id === work.kind);
  if (chosen !== undefined) return <Chosen pr={pr} def={chosen} send={send} onOpenLink={onOpenLink} />;
  return asked === undefined ? null : <Answer asked={asked} />;
}

/** What he asked of this pull request: one row each, his words in full, and what was kept under them. */
function Asks({ pr }: { pr: ReviewPr }) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  return (
    <ul className="pr-asked" aria-label="Asked">
      {work.asks.map((one) => {
        const running = one.run.state === "running";
        const kept = one.run.state === "done" ? one.run : undefined;
        const stale = kept !== undefined && kept.head !== pr.head;
        return (
          <li key={one.id} className="pr-asked__row" aria-label={one.words} data-selected={work.kind === one.id || undefined}>
            <button type="button" className="pr-asked__words" aria-pressed={work.kind === one.id} onClick={() => store.show(pr.key, one.id)}>
              {one.words}
            </button>
            {running ? (
              <span className="pr-review__mark" data-checks="running">
                <PrMark Glyph={LoaderCircle} said="Answering" tone="running" />
              </span>
            ) : one.run.state === "failed" ? (
              <PrMark Glyph={CircleX} said={`Failed. ${one.run.said}`} tone="failed" />
            ) : kept === undefined ? null : (
              <PrMark
                Glyph={stale ? RefreshCw : History}
                said={stale ? `Out of date. Answered on ${kept.head.slice(0, 7)}, the pull request is at ${pr.head.slice(0, 7)}` : `Kept. Answered ${agoOf(new Date(kept.at).toISOString())} on ${kept.head.slice(0, 7)}`}
              />
            )}
            <Tooltip label="Ask again, for this pull request">
              <Button variant="ghost" size="sm" iconOnly aria-label="Run again" disabled={running} onClick={() => store.spend("Ask", () => store.askAgain(pr.key, one.id))}>
                <RotateCw size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
            <Tooltip label="Remove this question">
              <Button variant="ghost" size="sm" iconOnly aria-label="Remove this question" onClick={() => store.removeAsk(pr.key, one.id)}>
                <X size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          </li>
        );
      })}
    </ul>
  );
}

/** What he asked, and what came back. Its keeping, Run again and Remove are on its row in the panel above. */
function Answer({ asked }: { asked: Asked }) {
  const running = asked.run.state === "running";
  const kept = asked.run.state === "done";
  const body = asked.answer;
  return (
    <div className="pr-what" role="region" aria-label="Answer">
      <header className="pr-what__head">
        <h3 className="pr-what__title pr-what__words">{asked.words}</h3>
        <span className="pr-what__source" data-by="agent">
          <PrMark Glyph={Bot} said="Answered by an agent when you asked" />
        </span>
      </header>
      {running ? (
        <p className="pr-results__running">
          <span className="pr-review__mark" data-checks="running">
            <PrMark Glyph={LoaderCircle} said="Answering" tone="running" />
          </span>
        </p>
      ) : asked.run.state === "failed" ? (
        <RunFailed said={asked.run.said} />
      ) : !kept ? null : body === undefined ? (
        <p className="pr-results__clean">
          <PrMark Glyph={Check} said="Nothing to add" tone="success" />
        </p>
      ) : (
        <Prose text={body} />
      )}
    </div>
  );
}

function Chosen({ pr, def, send, onOpenLink }: { pr: ReviewPr; def: ToolDef; send: Send; onOpenLink: (address: string) => void }) {
  const store = useReviewStore()!;
  const work = useReviews().work[pr.key] ?? NO_WORK;
  const run = work.explains[def.id];
  const running = run?.state === "running";
  const kept = run?.state === "done" ? run : undefined;
  const stale = kept !== undefined && kept.head !== pr.head;
  const source = SOURCE_MARK[def.source];
  const account: Account | undefined = work.accounts[def.id];
  return (
    <div className="pr-what" role="region" aria-label={def.name} data-source={def.source}>
      <header className="pr-what__head">
        <h3 className="pr-what__title">{def.name}</h3>
        <PrMark Glyph={source.Glyph} said={source.said} />
        <span className="pr-what__source" data-by="agent">
          <PrMark Glyph={Bot} said="Explained by an agent when you chose it" />
        </span>
        <span className="pr-what__acts">
          {kept === undefined ? null : (
            <PrMark
              Glyph={stale ? RefreshCw : History}
              said={stale ? `Out of date. Explained on ${kept.head.slice(0, 7)}, the pull request is at ${pr.head.slice(0, 7)}` : `Kept. Explained ${agoOf(new Date(kept.at).toISOString())} on ${kept.head.slice(0, 7)}`}
            />
          )}
          <Tooltip label="Explain again, for this pull request">
            <Button variant="ghost" size="sm" iconOnly aria-label="Run again" disabled={running} onClick={() => store.spend(def.name, () => store.explainAgain(pr.key, def))}>
              <RotateCw size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </header>
      {running ? (
        <p className="pr-results__running">
          <span className="pr-review__mark" data-checks="running">
            <PrMark Glyph={LoaderCircle} said="Explaining" tone="running" />
          </span>
        </p>
      ) : run?.state === "failed" ? (
        <RunFailed said={run.said} />
      ) : kept === undefined ? null : def.id === "how-it-works" ? (
        <>
          <Prose text={pr.explanation.summary} />
          <div className="pr-themes">
            {pr.explanation.themes.map((theme, at) => (
              <ThemeBlock key={theme.id} pr={pr} theme={theme} number={at + 1} mode="explain" send={send} />
            ))}
          </div>
        </>
      ) : account === undefined ? (
        <p className="pr-results__clean">
          <PrMark Glyph={Check} said="Nothing to add" tone="success" />
        </p>
      ) : (
        <AccountView account={account} onOpenLink={onOpenLink} />
      )}
    </div>
  );
}

function AccountView({ account, onOpenLink }: { account: Account; onOpenLink: (address: string) => void }) {
  if (account.shape === "text") return <Prose text={account.body} />;
  if (account.shape === "change") {
    return (
      <ul className="pr-before-after" aria-label="Before and after">
        {account.rows.map((row) => (
          <li key={row.what} className="pr-before-after__row" aria-label={row.what}>
            <h4 className="pr-before-after__what">{row.what}</h4>
            <p className="pr-before-after__before" data-side="before">
              <span className="pr-before-after__tag mono">Before</span>
              {row.before}
            </p>
            <p className="pr-before-after__after" data-side="after">
              <span className="pr-before-after__tag mono">After</span>
              {row.after}
            </p>
          </li>
        ))}
      </ul>
    );
  }
  if (account.shape === "steps") {
    return (
      <ol className="pr-steps" aria-label="Steps">
        {account.steps.map((step, at) => (
          <li key={at} className="pr-steps__step" aria-label={`Step ${at + 1}`}>
            <span className="pr-steps__n mono">{at + 1}</span>
            <div className="pr-steps__body">
              <p className="pr-steps__do">
                {step.do}
                {step.from === undefined ? null : step.from.from === "description" ? (
                  <PrMark Glyph={FileText} said="From the description" />
                ) : (
                  <Tooltip label={`From ${step.from.path}`}>
                    <Button variant="ghost" size="sm" iconOnly aria-label={`Open ${step.from.path}`} onClick={() => onOpenLink(step.from!.from === "doc" ? step.from!.address : "")}>
                      <BookOpen size={16} strokeWidth={2} aria-hidden />
                    </Button>
                  </Tooltip>
                )}
              </p>
              {step.code === undefined ? null : <Prose text={`\`\`\`bash\n${step.code}\n\`\`\``} />}
              <p className="pr-steps__look" role="note" aria-label={`Look for: ${step.look}`}>
                <PrMark Glyph={Eye} said="Look for" />
                {step.look}
              </p>
            </div>
          </li>
        ))}
      </ol>
    );
  }
  return (
    <div className="pr-reason">
      <Prose text={account.problem} />
      <Prose text={account.cost} />
      {account.links.length === 0 ? null : (
        <ul className="pr-reason__links" aria-label="Linked">
          {account.links.map((one) => (
            <li key={one.address}>
              <Button variant="ghost" size="sm" onClick={() => onOpenLink(one.address)}>
                <ExternalLink size={16} strokeWidth={2} aria-hidden />
                {one.label}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

const NOBODY = () => undefined;
const NONE: readonly never[] = [];

/**
 * His own question about the open pull request, in a compact box at the foot of the reader: **the Sessions box** (`SessionComposer`),
 * text only and on one row, as a Job's Commander has it. ↵ sends and ⇧↵ breaks the line. What is typed is kept per pull request
 * until it is sent. **It asks first**, since an answer spends tokens, and Cancel gives the words back.
 */
export function AskComposer({ pr }: { pr: ReviewPr }) {
  const store = useReviewStore()!;
  return (
    <div className="pr-ask-bar">
      <Tooltip label="Ask about this pull request" shortcut={capOf(REVIEW_KEYS.ask)}>
        <Kbd>{capOf(REVIEW_KEYS.ask)}</Kbd>
      </Tooltip>
      <SessionComposer
        working={false}
        bare
        label="Ask about this PR"
        send="secondary"
        slim
        model={null}
        effort={null}
        models={NONE}
        efforts={NONE}
        onTune={NOBODY}
        commands={NONE}
        taggable={NONE}
        tags={NONE}
        onTags={NOBODY}
        drawn={NONE}
        onDraw={NOBODY}
        onRemoveDrawn={NOBODY}
        draftKey={`pr-review:${pr.key}`}
        onSend={(sent) => new Promise<boolean>((resolve) => store.spend("Ask", () => (store.ask(pr.key, sent.text), resolve(true)), () => resolve(false)))}
      />
    </div>
  );
}
