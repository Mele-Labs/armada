// A Job's document on the phone: its newest kept revision, read through the Gateway as text and drawn by the page reader Bridge
// uses (`@armada/screens/src/document-page`), which keeps only what its allow list names. The page is put into one element by hand
// and its CSS adopted under that element's class; the real sketch is drawn at each `<armada-sketch>`. Bridge's notes and margin
// are not here: the phone answers a revision with Approve or Send back.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import { createPortal } from "react-dom";
import { ApprovalAnswers, SketchScene, type SketchAnswer } from "@armada/components";
import { PAGE_ROOT, readPage, sceneWithKept } from "@armada/screens/src/document-page";
import { documentAnswerOf } from "@armada/screens/src/thread-cards";

import { macOf, start, useFleet } from "./fleet";
import { actsOn, attemptPage, readShown, useJob, type Shown } from "./jobState";
import { WithKeptStyles } from "./keptStyles";
import { go, jobPath } from "./router";
import { documentOf, sessionOfThread } from "./thread";
import { Band, Trouble } from "./ui";

type Mount = {
  el: HTMLElement;
  index: number;
  title: string;
};

export function DocumentView({ origin, id, page }: { origin: string; id: string; page: string }) {
  useFleet();
  useEffect(() => void start(), []);
  const machine = macOf(origin)?.machine;
  const [read, reread] = useJob(machine, id);
  const session = read.thread === undefined ? undefined : sessionOfThread(read.thread);
  const doc = session === undefined ? undefined : documentOf(session, page);
  const newest = doc?.revisions[doc.revisions.length - 1];
  const [shown, setShown] = useState<Shown | undefined>(undefined);
  const [says, setSays] = useState("");
  const [mounts, setMounts] = useState<readonly Mount[]>([]);
  const el = useRef<HTMLDivElement>(null);

  // The newest revision, read again whenever the Job keeps another.
  useEffect(() => {
    if (machine === undefined || newest === undefined) return;
    let gone = false;
    void readShown(machine, id, newest).then((got) => !gone && setShown(got));
    return () => {
      gone = true;
    };
  }, [
    machine?.origin,
    id,
    newest,
  ]);

  const parsed = useMemo(
    () => (shown?.kind === "text" ? readPage(shown.text) : undefined),
    [
      shown,
    ],
  );
  const title = doc?.title ?? page.slice(page.lastIndexOf("/") + 1);
  const keptStyles = doc?.keptStyles;
  const scenes = useMemo(
    () => parsed?.sketches.map((one) => sceneWithKept(one.title, one.scene, keptStyles)) ?? [],
    [
      parsed,
      keptStyles,
    ],
  );

  // The page's CSS reaches only the page: every rule was put under the page's class when it was read.
  useEffect(() => {
    if (parsed === undefined) return;
    const sheet = new CSSStyleSheet();
    sheet.replaceSync(parsed.css);
    document.adoptedStyleSheets = [
      ...document.adoptedStyleSheets,
      sheet,
    ];
    return () => {
      document.adoptedStyleSheets = document.adoptedStyleSheets.filter((one) => one !== sheet);
    };
  }, [
    parsed,
  ]);

  useLayoutEffect(() => {
    const into = el.current;
    if (into === null || parsed === undefined) return;
    into.replaceChildren(
      ...[
        ...parsed.root.childNodes,
      ].map((one) => one.cloneNode(true)),
    );
    // An address outside the page opens beside the app and not over it.
    for (const link of into.querySelectorAll("a[href^='http']")) {
      link.setAttribute("target", "_blank");
      link.setAttribute("rel", "noopener noreferrer");
    }
    // A table wider than the screen scrolls inside its own box, and the page stays as wide as the screen.
    for (const table of into.querySelectorAll("table")) {
      if (table.closest(".armada-doc__sketch-mount") !== null) continue;
      const box = document.createElement("div");
      box.className = "pk-doc__scroll";
      box.tabIndex = 0;
      box.setAttribute("role", "region");
      box.setAttribute("aria-label", table.querySelector("caption")?.textContent ?? "Table");
      table.replaceWith(box);
      box.append(table);
    }
    setMounts(
      [
        ...into.querySelectorAll<HTMLElement>(".armada-doc__sketch-mount"),
      ].flatMap((mount) => {
        const figure = mount.parentElement;
        return figure === null
          ? []
          : [
              {
                el: mount,
                index: Number(figure.dataset.sketch),
                title: figure.dataset.title ?? "Sketch",
              },
            ];
      }),
    );
  }, [
    parsed,
  ]);

  // A link in the page scrolls to an address in it. Any other address is left alone.
  const onClick = (event: MouseEvent<HTMLElement>) => {
    const link = (event.target as Element).closest("a");
    if (link === null) return;
    const href = link.getAttribute("href") ?? "";
    if (!href.startsWith("#")) return;
    event.preventDefault();
    el.current?.querySelector(`#${CSS.escape(href.slice(1))}`)?.scrollIntoView({
      block: "start",
    });
  };

  const answer = (given: SketchAnswer) => {
    if (machine === undefined || session === undefined) return;
    setSays("");
    void attemptPage(
      () => actsOn(machine, reread).answerDocument(documentAnswerOf(session, page, given)),
      machine,
      reread,
    ).then((refused) => setSays(refused ?? ""));
  };

  return (
    <section className="pk-screen" aria-label={title}>
      <Band title={title} onBack={() => go(jobPath(origin, id))} back="Back to the Job" />
      <div className="pk-body pk-doc">
        {shown?.kind === "refused" ? <Trouble says={shown.says} /> : null}
        {doc === undefined && read.thread !== undefined ? (
          <Trouble says="That Job did not write that page." />
        ) : null}
        {read.thread === undefined ? <Trouble says={read.error ?? ""} /> : null}
        <WithKeptStyles machine={machine}>
          <div
            ref={el}
            className={`${PAGE_ROOT} pk-doc__page`}
            role="article"
            aria-label={title}
            onClick={onClick}
          />
          {mounts.map((one) =>
            createPortal(
              <SketchScene scene={scenes[one.index]} title={one.title} />,
              one.el,
              String(one.index),
            ),
          )}
        </WithKeptStyles>
        {doc === undefined || (!doc.waits && doc.settled === undefined) ? null : (
          <div className="pk-doc__answers">
            <ApprovalAnswers
              id={page}
              keep={`pocket:${origin}:${id}:document:${page}`}
              {...(doc.settled === undefined
                ? {}
                : {
                    settled: doc.settled,
                  })}
              open={doc.waits}
              onAnswer={(_, given) => answer(given)}
            />
            <Trouble says={says} />
          </div>
        )}
      </div>
    </section>
  );
}
