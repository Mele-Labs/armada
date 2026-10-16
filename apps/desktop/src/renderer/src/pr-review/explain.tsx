// The explanations: one choice under the description, in the stage's own overview. **Each is its own question** (how it
// works, what it changes for him as a user, how to test it, why it was needed), and an agent answers one only when he
// first chooses it. The answer is kept per commit, as a tool's result is, so choosing another and coming back shows it
// at once. Explanations are defined in the same three places the tools are, so a repository can add its own.

import { useState } from "react";
import { Bot, Check, ExternalLink, Eye, History, LoaderCircle, RefreshCw, RotateCw, SquareTerminal, X } from "lucide-react";
import { Button, Prose, PrMark, Tabs, Textarea, Tooltip, agoOf, forgetKept, sendsOn } from "@armada/components";

import type { useSendFindings } from "./acts";
import { SOURCE_MARK } from "./marks";
import type { Account, Asked, ReviewPr, ToolDef } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { ThemeBlock } from "./themes";
import { bySession, onlyExplanations, toolsOf } from "./tools";

type Send = ReturnType<typeof useSendFindings>;

const ASK = "ask";

/**
 * The choice and what was chosen, under the description. **The strip is the explanations, then Ask**, which is not an
 * explanation but a box for his own question, as Natural Search is on the left. **A question he sent is kept in a small
 * panel above the strip**, an index of what he asked of this pull request, and is not drawn where he asked none.
 */
export function Explainer({ pr, send, onOpenLink }: { pr: ReviewPr; send: Send; onOpenLink: (address: string) => void }) {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const work = reviews.work[pr.key] ?? NO_WORK;
  const kinds = onlyExplanations(toolsOf(reviews, pr.manifestId));
  const chosen = kinds.find((one) => one.id === work.kind);
  const asked = work.asks.find((one) => one.id === work.kind);
  return (
    <section className="pr-explainer" aria-label="Explanation">
      {work.asks.length === 0 ? null : <Asks pr={pr} />}
      <div className="pr-explainer__pick">
        <Tabs
          items={[
            ...kinds.map((one) => ({ id: one.id, label: one.name, hint: one.checks })),
            { id: ASK, label: "Ask", hint: "Ask about this pull request in your own words" },
          ]}
          value={work.kind ?? ""}
          onChange={(id) => {
            const def = kinds.find((one) => one.id === id);
            if (def === undefined) store.show(pr.key, id);
            else store.explain(pr.key, def);
          }}
        />
      </div>
      {work.kind === ASK ? <AskBox pr={pr} /> : null}
      {chosen === undefined ? null : <Chosen pr={pr} def={chosen} send={send} onOpenLink={onOpenLink} />}
      {asked === undefined ? null : <Answer pr={pr} asked={asked} />}
    </section>
  );
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
            ) : kept === undefined ? null : (
              <PrMark
                Glyph={stale ? RefreshCw : History}
                said={stale ? `Out of date. Answered on ${kept.head.slice(0, 7)}, the pull request is at ${pr.head.slice(0, 7)}` : `Kept. Answered ${agoOf(new Date(kept.at).toISOString())} on ${kept.head.slice(0, 7)}`}
              />
            )}
            <Tooltip label="Ask again, for this pull request">
              <Button variant="ghost" size="sm" iconOnly aria-label="Run again" disabled={running} onClick={() => store.askAgain(pr.key, one.id)}>
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

/** His own question, written where the answer will be. It runs only when he sends, because it costs. */
function AskBox({ pr }: { pr: ReviewPr }) {
  const store = useReviewStore()!;
  const keep = `pr-review:ask:${pr.key}`;
  const [words, setWords] = useState("");
  const send = () => {
    if (words.trim() === "") return;
    store.ask(pr.key, words);
    forgetKept(keep);
    setWords("");
  };
  return (
    <form
      className="pr-what pr-ask"
      aria-label="Ask"
      onSubmit={(event) => {
        event.preventDefault();
        send();
      }}
    >
      <Textarea
        rows={3}
        autoFocus
        keep={keep}
        aria-label="Ask about this pull request"
        value={words}
        onChange={(event) => setWords(event.target.value)}
        onKeyDown={(event) => {
          if (sendsOn(event) && !event.nativeEvent.isComposing) {
            event.preventDefault();
            send();
          }
        }}
      />
      <Tooltip label="An agent reads this pull request and answers, once, when you send" asChild>
        <Button variant="secondary" size="sm" type="submit" disabled={words.trim() === ""}>
          Ask
        </Button>
      </Tooltip>
    </form>
  );
}

/** What he asked, and what came back. Its keeping, Run again and Remove are on its row in the panel above. */
function Answer({ pr, asked }: { pr: ReviewPr; asked: Asked }) {
  const running = asked.run.state === "running";
  const kept = asked.run.state === "done";
  const body = pr.answers.find((one) => new RegExp(one.when, "i").test(asked.words))?.body;
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
  const fromSession = bySession(pr, def);
  const stale = !fromSession && kept !== undefined && kept.head !== pr.head;
  const source = SOURCE_MARK[def.source];
  const by = fromSession ? { Glyph: SquareTerminal, said: "Written by the Session that made this pull request" } : { Glyph: Bot, said: "Explained by an agent when you chose it" };
  const account: Account | undefined = pr.accounts[def.id];
  return (
    <div className="pr-what" role="region" aria-label={def.name} data-source={def.source}>
      <header className="pr-what__head">
        <h3 className="pr-what__title">{def.name}</h3>
        <PrMark Glyph={source.Glyph} said={source.said} />
        <span className="pr-what__source" data-by={fromSession ? "session" : "agent"}>
          <PrMark Glyph={by.Glyph} said={by.said} />
        </span>
        <span className="pr-what__acts">
          {kept === undefined || fromSession ? null : (
            <PrMark
              Glyph={stale ? RefreshCw : History}
              said={stale ? `Out of date. Explained on ${kept.head.slice(0, 7)}, the pull request is at ${pr.head.slice(0, 7)}` : `Kept. Explained ${agoOf(new Date(kept.at).toISOString())} on ${kept.head.slice(0, 7)}`}
            />
          )}
          {fromSession ? null : (
            <Tooltip label="Explain again, for this pull request">
              <Button variant="ghost" size="sm" iconOnly aria-label="Run again" disabled={running} onClick={() => store.explainAgain(pr.key, def)}>
                <RotateCw size={16} strokeWidth={2} aria-hidden />
              </Button>
            </Tooltip>
          )}
        </span>
      </header>
      {running ? (
        <p className="pr-results__running">
          <span className="pr-review__mark" data-checks="running">
            <PrMark Glyph={LoaderCircle} said="Explaining" tone="running" />
          </span>
        </p>
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
              <p className="pr-steps__do">{step.do}</p>
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
