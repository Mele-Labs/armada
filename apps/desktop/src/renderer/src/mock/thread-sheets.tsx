// Two pages beside the spacious thread's walk, over the app at `?sheet=`: `thread-states` draws a Job in each state its
// thread can be in, today's look beside the take (scenario `thread-states`); `ripple` lists each shared piece the take
// touches, where else it is drawn, and that piece's story before and after. The take's column is the ancestor its rules key
// on (`spacious-thread.css`) and the context its thread reads, so both looks sit on one page.

import { useEffect, useRef, useState, type ReactElement, type ReactNode } from "react";
import { createPortal } from "react-dom";
import {
  EyeOff,
  ListPlus,
  MessageSquareOff,
  MessageSquarePlus,
  MessageSquareQuote,
  type LucideIcon,
} from "lucide-react";
import { SpaciousTake } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { StoryBoundary, storyElement } from "../annotate/story";
import { askToOpenSession } from "../open-session";
import { SessionView } from "../sessions";
import { useSessions, useSessionsDraft } from "../sessions-draft";
import { storyFile } from "./story-files";
import "./thread-sheets.css";

const HELD = {
  held: {
    state: "none",
  },
  onWant: () => {},
} as const;

const GOES = {
  onGoTo: () => {},
  onOpenLink: () => {},
  onOpenCheck: () => {},
  onOpenLoadout: () => {},
};

/**
 * Today's look, or the take: the take's column carries what its rules key on and, for a Job, what its thread reads. A piece
 * drawn outside a Job gets the flag alone, as the app would give it: only the Job view hands its thread the take.
 */
function Take({
  spacious,
  job = false,
  children,
}: {
  spacious: boolean;
  job?: boolean;
  children: ReactNode;
}) {
  return spacious ? (
    <div className="thread-sheet__take" data-thread-take="spacious">
      {job ? <SpaciousTake.Provider value>{children}</SpaciousTake.Provider> : children}
    </div>
  ) : (
    <div className="thread-sheet__take">{children}</div>
  );
}

function JobCell({ session, width }: { session: Session; width?: number }) {
  return (
    <div
      className="deck thread-sheet__deck"
      style={
        width === undefined
          ? undefined
          : {
              width,
            }
      }
    >
      <section
        className="deck-pane"
        data-frame=""
        aria-label={`Job: ${session.title ?? session.id}`}
      >
        <SessionView
          session={session}
          goes={GOES}
          onOpen={askToOpenSession}
          held={HELD}
          deck={{
            ledger: null,
            back: null,
            acts: [],
          }}
        />
      </section>
    </div>
  );
}

const STATES: readonly {
  name: string;
  id: string;
  width?: number;
  focus?: true;
}[] = [
  {
    name: "A long message each way",
    id: "st1",
  },
  {
    name: "Twelve calls in a row",
    id: "st2",
  },
  {
    name: "Code",
    id: "st3",
  },
  {
    name: "A table",
    id: "st4",
  },
  {
    name: "A failed run",
    id: "st5",
  },
  {
    name: "Needs you",
    id: "st8",
  },
  {
    name: "A document waiting on Approve",
    id: "s58",
  },
  {
    name: "A walk",
    id: "st6",
  },
  {
    name: "Thinking",
    id: "s91",
  },
  {
    name: "Making calls",
    id: "s94",
  },
  {
    name: "Reasoned, then answered",
    id: "s92",
  },
  {
    name: "Nothing left to do",
    id: "s93",
  },
  {
    name: "A thread like this one",
    id: "st9",
  },
  {
    name: "Mixed calls between two messages",
    id: "st11",
  },
  {
    name: "A slot leased and released",
    id: "st12",
  },
  {
    name: "A long code block, scrolled into",
    id: "st13",
  },
  {
    name: "Pictures and a file waiting to send",
    id: "st14",
  },
  {
    name: "Pictures, a video and a doc",
    id: "st10",
  },
  {
    name: "An empty thread",
    id: "st7",
  },
  {
    name: "A narrow pane, 360 wide",
    id: "st1",
    width: 360,
  },
  {
    name: "Keyboard focus on a paragraph",
    id: "st4",
    focus: true,
  },
];

// What neither page draws, so a state left out is named rather than taken as covered.
const NOT_SHOWN: readonly string[] = [
  "Light and the other themes: every row is the dark default",
  "Pocket's Job page, which draws the same thread and does not load the take",
  "Rows arriving while the agent works, and the thread following them",
  "The Job's side panes and the review sheet beside the thread",
  "Sketch, plan and question cards, and a permission prompt",
  "Subagents, a compacted conversation and a forked thread",
  "The Aa panel's text sizes and spacing",
  "Widths other than half this page and 360",
];

// The quick bar's Save and Hide, each pair as the message box would draw it, so the choice is made by looking.
const QUICK_ICONS: readonly {
  name: string;
  save: LucideIcon;
  hide: LucideIcon;
}[] = [
  {
    name: "Quote bubble and eye",
    save: MessageSquareQuote,
    hide: EyeOff,
  },
  {
    name: "Plus bubble and struck bubble",
    save: MessageSquarePlus,
    hide: MessageSquareOff,
  },
  {
    name: "Today's list and eye",
    save: ListPlus,
    hide: EyeOff,
  },
];

function QuickIcons() {
  return (
    <section className="thread-sheet__row" aria-label="Quick bar icons">
      <h2 className="thread-sheet__name">Quick bar icons</h2>
      <div className="thread-sheet__icons">
        {QUICK_ICONS.map(({ name, save: Save, hide: Hide }) => (
          <figure key={name} className="thread-sheet__icon-take" data-take={name}>
            <div className="thread-sheet__box">
              <span className="thread-sheet__glyph" title="Save as quick reply">
                <Save size={15} strokeWidth={2} aria-hidden />
              </span>
              <span className="thread-sheet__glyph" title="Hide quick replies">
                <Hide size={15} strokeWidth={2} aria-hidden />
              </span>
            </div>
            <figcaption>{name}</figcaption>
          </figure>
        ))}
      </div>
    </section>
  );
}

function StateSheet() {
  const sessions = useSessions();
  const draft = useSessionsDraft();
  const focusRow = useRef<HTMLElement>(null);
  // Focus is one place on a page: it starts on the take's paragraph, as Tab leaves it, and Tab moves it on.
  useEffect(() => {
    const at = window.setTimeout(() => {
      focusRow.current
        ?.querySelector<HTMLElement>("[data-thread-take] .armada-prose [data-pick]")
        ?.focus({
          preventScroll: true,
        });
    }, 400);
    return () => window.clearTimeout(at);
  }, []);
  if (draft === undefined) return null;
  return (
    <>
      <QuickIcons />
      <div className="thread-sheet__heads" aria-hidden>
        <span>Today</span>
        <span>Spacious</span>
      </div>
      {STATES.map((state) => {
        const session = sessions.find((one) => one.id === state.id);
        if (session === undefined) return null;
        return (
          <section
            key={state.name}
            ref={state.focus === true ? focusRow : undefined}
            className="thread-sheet__row"
            aria-label={state.name}
          >
            <h2 className="thread-sheet__name">{state.name}</h2>
            <div className="thread-sheet__pair">
              {[
                false,
                true,
              ].map((spacious) => (
                <Take key={String(spacious)} spacious={spacious} job>
                  <JobCell
                    session={session}
                    {...(state.width === undefined
                      ? {}
                      : {
                          width: state.width,
                        })}
                  />
                </Take>
              ))}
            </div>
          </section>
        );
      })}
      <section className="thread-sheet__row" aria-label="Not shown here">
        <h2 className="thread-sheet__name">Not shown here</h2>
        <ul className="thread-sheet__missing">
          {NOT_SHOWN.map((one) => (
            <li key={one}>{one}</li>
          ))}
        </ul>
      </section>
    </>
  );
}

function StoryTile({ file, story }: { file: string; story: string }) {
  const [drawn, setDrawn] = useState<ReactElement | null>(null);
  useEffect(() => {
    let live = true;
    void storyFile(file).then((module) => {
      if (live && module !== null) setDrawn(storyElement(module, story));
    });
    return () => {
      live = false;
    };
  }, [
    file,
    story,
  ]);
  return (
    <div className="thread-sheet__tile">
      <StoryBoundary>{drawn}</StoryBoundary>
    </div>
  );
}

// From the code graph, the files outside a Job's own thread that draw each piece. Tests and stories are left out.
const PIECES: readonly {
  name: string;
  places: readonly string[];
  count: number;
  file?: string;
  story?: string;
}[] = [
  {
    name: "Composer",
    count: 10,
    places: [
      "Operator panel",
      "Judge tab",
      "Pull request explain",
      "Report box",
      "Console bays",
      "Workflow manager, twice",
      "A pane's custom card and body",
      "Pocket's Job page",
    ],
    file: "packages/components/src/compositions/SessionComposer/SessionComposer.stories.tsx",
    story: "Blank",
  },
  {
    name: "Prose",
    count: 16,
    places: [
      "Pull request panel and tabs",
      "Pull request review: reader, lines, explain, themes",
      "Issues reader",
      "File preview",
      "Viewer notes",
      "Job notes",
      "A subagent's turns",
      "An artifact",
      "Drone turns",
      "Loadout item panel",
      "Pocket's Job and Shown pages",
    ],
    file: "packages/components/src/primitives/Prose/Prose.stories.tsx",
    story: "AListAndAHeading",
  },
  {
    name: "Button",
    count: 226,
    places: [
      "Every surface in Bridge and Pocket",
    ],
    file: "packages/components/src/primitives/Button/Button.stories.tsx",
    story: "Secondary",
  },
  {
    name: "Thread",
    count: 5,
    places: [
      "Cockpit call card",
      "A subagent's thread",
      "A pane's custom card and body",
      "Pocket's Job page",
    ],
    file: "packages/components/src/compositions/SessionThread/SessionThread.stories.tsx",
    story: "ToolCallsFoldIntoOneRow",
  },
  {
    name: "Needs you",
    count: 0,
    places: [],
    file: "packages/components/src/compositions/NeedsYouPanel/NeedsYouPanel.stories.tsx",
    story: "TheTriggerIsNamedNeedsYou",
  },
  {
    name: "Job frame",
    count: 1,
    places: [
      "Pocket's Job page, which does not load the take",
    ],
  },
  {
    name: "Tokens and shared stylesheets",
    count: 0,
    places: [],
  },
];

function RippleSheet() {
  return (
    <>
      <p className="thread-sheet__lead">
        Every rule of the take is scoped to a Job's own thread, so each place below draws as it does
        today.
      </p>
      <div className="thread-sheet__heads" aria-hidden>
        <span>Before</span>
        <span>After</span>
      </div>
      {PIECES.map((piece) => (
        <section key={piece.name} className="thread-sheet__row" aria-label={piece.name}>
          <h2 className="thread-sheet__name">
            {piece.name}
            <span className="thread-sheet__count">
              {piece.count === 0
                ? "Only in a Job's thread"
                : `${piece.count} places outside a Job's thread`}
            </span>
          </h2>
          {piece.places.length === 0 ? null : (
            <ul className="thread-sheet__places">
              {piece.places.map((place) => (
                <li key={place}>{place}</li>
              ))}
            </ul>
          )}
          {piece.file === undefined || piece.story === undefined ? null : (
            <div className="thread-sheet__pair">
              {[
                false,
                true,
              ].map((spacious) => (
                <Take key={String(spacious)} spacious={spacious}>
                  <StoryTile file={piece.file!} story={piece.story!} />
                </Take>
              ))}
            </div>
          )}
        </section>
      ))}
    </>
  );
}

/** The sheet `?sheet=` names, over the app; nothing without it. */
export function ThreadSheets() {
  const sheet = new URLSearchParams(window.location.search).get("sheet");
  if (sheet !== "thread-states" && sheet !== "ripple") return null;
  return createPortal(
    <main
      className="thread-sheet"
      aria-label={sheet === "ripple" ? "Where the take reaches" : "Thread states"}
    >
      {sheet === "ripple" ? <RippleSheet /> : <StateSheet />}
    </main>,
    document.body,
  );
}
