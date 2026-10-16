// The review's Flight control: one tile per review tool, keyed 1 to 9, in the Cockpit's own tile language. **A
// tool runs for the open pull request only when pressed**, because an agent run is expensive, and shows running on
// its tile. The tools are definitions from three places, as workflows are, and each tile carries its place as a
// small glyph: Armada's, his Kit's (`~/.armada/review-tools/`), or the repository's (`.armada/review-tools/`).
// The last tile adds one. Mock only: it records the tool and writes no file.

import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { History, LoaderCircle, RefreshCw, SquarePlus } from "lucide-react";
import { Button, Input, Kbd, PrMark, Radio, RadioGroup, Sheet, Textarea, Tooltip, agoOf, forgetKept } from "@armada/components";
import { useAtFloor } from "@armada/shell";

import { SOURCE_MARK, TOOL_MARK } from "./marks";
import type { ReviewPr, ToolDef } from "./model";
import { NO_WORK, useReviewStore, useReviews } from "./store";
import { onlyTools, toolsOf } from "./tools";

/** What a tile says under its glyph, where the tool's name is long. */
const SHORT: Record<string, string> = { "api-breaking-changes": "API breaks" };

const WHOSE = { armada: "Armada's", kit: "the Kit's", repository: "the repository's" } as const;

const typing = (target: EventTarget | null): boolean => target instanceof HTMLElement && (target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName));

/** The tiles, as a block for a Flight control panel to hold. `leave` is the Cockpit's mode tile, drawn last where there is one. */
export function ReviewTools({ leave }: { leave?: { pressed: boolean; press: () => void; icon: ReactNode } }) {
  const store = useReviewStore()!;
  const reviews = useReviews();
  const pr = reviews.prs.find((one) => one.key === reviews.open);
  const work = pr === undefined ? NO_WORK : (reviews.work[pr.key] ?? NO_WORK);
  const tools = onlyTools(toolsOf(reviews, pr?.manifestId));
  const [adding, setAdding] = useState(false);

  const press = (tool: ToolDef) => {
    if (pr === undefined) return;
    store.runTool(pr.key, tool);
  };

  // The keys 1 to 9 press the tiles, where no box is being typed in.
  useEffect(() => {
    const down = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey || event.shiftKey || typing(event.target) || adding) return;
      const at = Number(event.key) - 1;
      if (!Number.isInteger(at) || at < 0 || at > 8) return;
      if (at < tools.length) press(tools[at]!);
      else if (at === tools.length) setAdding(true);
      else return;
      event.preventDefault();
    };
    window.addEventListener("keydown", down);
    return () => window.removeEventListener("keydown", down);
  });

  const count = tools.length + 1 + (leave === undefined ? 0 : 1);
  return (
    <>
      <div className="deck-controls__block pr-tools" data-n={count} style={{ "--n": count } as React.CSSProperties}>
        {tools.map((tool, at) => {
          const run = work.runs[tool.id];
          const running = run?.state === "running";
          const blocked = pr === undefined;
          const kept = run?.state === "done" && tool.agent ? run : undefined;
          // A result is kept for the commit it ran on. A newer commit marks it out of date, and it is still shown.
          const stale = kept !== undefined && pr !== undefined && kept.head !== pr.head;
          const mark = TOOL_MARK[tool.mark];
          const source = SOURCE_MARK[tool.source];
          return (
            <Tooltip key={tool.id} label={tool.name} shortcut={String(at + 1)} asChild>
              <button type="button" className="deck-tile pr-tool" aria-label={tool.name} aria-pressed={work.view === tool.id} aria-disabled={blocked || undefined} aria-busy={running || undefined} data-tool={tool.mark} data-source={tool.source} data-stale={stale || undefined} onClick={() => press(tool)}>
                {running ? <LoaderCircle size={16} aria-hidden="true" className="pr-tool__spin" /> : <mark.Glyph size={16} aria-hidden="true" />}
                <span className="deck-tile__text pr-tool__text">{SHORT[tool.id] ?? tool.name}</span>
                <Kbd className="deck-tile__key" aria-hidden="true">
                  {at + 1}
                </Kbd>
                {kept === undefined ? null : (
                  <span className="pr-tool__kept">
                    <PrMark
                      Glyph={stale ? RefreshCw : History}
                      said={stale ? `Out of date. Ran on ${kept.head.slice(0, 7)}, the pull request is at ${pr!.head.slice(0, 7)}` : `Kept. Ran ${agoOf(new Date(kept.at).toISOString())} on ${kept.head.slice(0, 7)}`}
                    />
                  </span>
                )}
                <span className="pr-tool__source" data-source={tool.source}>
                  <PrMark Glyph={source.Glyph} said={tool.replaces === undefined ? source.said : `${source.said}. Replaces ${WHOSE[tool.replaces]} ${tool.name}`} />
                </span>
              </button>
            </Tooltip>
          );
        })}
        <Tooltip label="Add a tool" shortcut={String(tools.length + 1)} asChild>
          <button type="button" className="deck-tile pr-tool" aria-label="Add a tool" onClick={() => setAdding(true)}>
            <SquarePlus size={16} aria-hidden="true" />
            <span className="deck-tile__text pr-tool__text">Add a tool</span>
            <Kbd className="deck-tile__key" aria-hidden="true">
              {tools.length + 1}
            </Kbd>
          </button>
        </Tooltip>
        {leave === undefined ? null : (
          <Tooltip label="PR Review" asChild>
            <button type="button" className="deck-tile" data-mode aria-label="PR Review" aria-pressed={leave.pressed} onClick={leave.press}>
              {leave.icon}
              <span className="deck-tile__text">PR Review</span>
            </button>
          </Tooltip>
        )}
      </div>
      <AddTool open={adding} pr={pr} onClose={() => setAdding(false)} />
    </>
  );
}

/** Take B's panel: the same tiles under the same band as the Cockpit's Flight control, at the foot of the review surface. */
export function ReviewFlight() {
  return (
    <div className="deck-foot">
      <section className="deck-panel deck-controls" aria-label="Flight controls" tabIndex={-1}>
        <header className="deck-band">Flight control</header>
        <ReviewTools />
      </section>
    </div>
  );
}

function AddTool({ open, pr, onClose }: { open: boolean; pr: ReviewPr | undefined; onClose: () => void }) {
  const store = useReviewStore()!;
  const floor = useAtFloor();
  const [name, setName] = useState("");
  const [checks, setChecks] = useState("");
  const [scope, setScope] = useState<"kit" | "repository">("kit");
  const [kind, setKind] = useState<ToolDef["kind"]>("tool");
  const ready = name.trim() !== "" && checks.trim() !== "";
  const keep = "pr-review:add-tool:checks";
  const save = () => {
    if (!ready) return;
    store.addTool({ kind, name, checks, scope, manifestId: pr?.manifestId ?? "" });
    forgetKept(keep);
    setName("");
    setChecks("");
    setScope("kit");
    setKind("tool");
    onClose();
  };
  return (
    <Sheet kind="review-tool" open={open} floating floor={floor} title="Add a tool" closeLabel="Close" closeBinding="Esc" onClose={onClose}>
      <form
        className="pr-add"
        aria-label="New tool"
        onSubmit={(event) => {
          event.preventDefault();
          save();
        }}
      >
        <RadioGroup label="Kind">
          <Tooltip label="Finds things on a pull request, as a tile in the Flight control" asChild>
            <span className="pr-add__choice">
              <Radio name="kind" checked={kind === "tool"} onChange={() => setKind("tool")}>
                Tool
              </Radio>
            </span>
          </Tooltip>
          <Tooltip label="Tells him one thing about a pull request, chosen under its description" asChild>
            <span className="pr-add__choice">
              <Radio name="kind" checked={kind === "explanation"} onChange={() => setKind("explanation")}>
                Explanation
              </Radio>
            </span>
          </Tooltip>
        </RadioGroup>
        <Input label="Name" value={name} onChange={(event) => setName(event.target.value)} />
        <Textarea rows={6} keep={keep} aria-label={kind === "tool" ? "What it checks" : "What it explains"} placeholder={kind === "tool" ? "What it checks" : "What it explains"} value={checks} onChange={(event) => setChecks(event.target.value)} />
        <RadioGroup label="Save to">
          <Tooltip label="~/.armada/review-tools/, every repository on this machine" asChild>
            <span className="pr-add__choice">
              <Radio name="scope" checked={scope === "kit"} onChange={() => setScope("kit")}>
                Kit
              </Radio>
            </span>
          </Tooltip>
          <Tooltip label={`.armada/review-tools/ in ${pr?.repository ?? "the repository"}, shared with everyone on it`} asChild>
            <span className="pr-add__choice">
              <Radio name="scope" checked={scope === "repository"} onChange={() => setScope("repository")}>
                Repository
              </Radio>
            </span>
          </Tooltip>
        </RadioGroup>
        <div className="pr-add__bar">
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" type="submit" disabled={!ready}>
            Save
          </Button>
        </div>
      </form>
    </Sheet>
  );
}
