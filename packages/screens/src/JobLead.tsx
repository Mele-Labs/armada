// The banner that leads a Job's Overview — a sentence, what it is holding up,
// and at most one act.
//
// **One copy, because the train already had one.** `plan-lead.tsx` is the scar
// that says why: a sentence spelled twice came back wrong the first time one
// of the two was fixed.

import { Prose, SkeletonText } from "@armada/components";
import type { ReactNode } from "react";

export type JobLeadProps = {
  said: string;
  because: string;
  /**
   * The Drone's own question, drawn through `Prose` above `because`. **Its
   * words are a model's markdown and Armada's are not**, so the two are never
   * one string: joined, Armada's tail would be parsed as markdown too.
   */
  asked?: string;
  /**
   * The Job already fixing what failed, leading the second line: its title,
   * a press where the shell can open it, then `rest`. #1673.
   */
  fix?: { title: string; rest: string; onOpen?: () => void };
  /** Colours the edge — the thing outstanding is found before a word is read. */
  tone?: "awaiting-review" | "completed-failed";
  /** How long the thing under it has waited, already rendered. Top right. */
  elapsed?: string;
  /** The one act, where the screen has somewhere to send a person. */
  act?: ReactNode;
  /**
   * What the lead is about, where a person answers it here: the Drone's
   * question, the command it was not given, or the review gate.
   *
   * **Inside this panel and not under it** — the owner, 30 Sep 2026:
   * *"This panel duplicates what is shown in the panel below. Why can't we
   * just have one panel?"* The boxes drop their own heading and their
   * restatement, because the lead has already said both.
   */
  waiting?: ReactNode;
  /**
   * Whether what the lead would say is still a guess — its quiet line, before
   * this Job's own read has answered. Bars stand where the sentence lands.
   */
  reading?: boolean;
};

export function JobLead({
  said,
  because,
  asked,
  fix,
  tone,
  elapsed,
  act,
  waiting,
  reading = false,
}: JobLeadProps) {
  return (
    <div className="armada-lead" data-tone={tone}>
      <div className="armada-lead__head">
        <div className="armada-lead__said">
          {/* **A heading, because it leads the destination.** Everything under
              it is subordinate to the one thing it names, and a reader moving by
              headings should land here first. */}
          {reading ? (
            // Two bars: the headline and the line under it.
            <SkeletonText widths={["30%", "55%"]} />
          ) : (
            <>
              <h2 className="armada-lead__headline">{said}</h2>
              {asked === undefined ? null : (
                <div className="armada-lead__asked">
                  <Prose text={asked} />
                </div>
              )}
              {because === "" && fix === undefined ? null : (
                <p className="armada-lead__because">
                  {fix === undefined ? null : (
                    <>
                      {fix.onOpen === undefined ? (
                        fix.title
                      ) : (
                        <button type="button" className="armada-lead__opens" onClick={fix.onOpen}>
                          {fix.title}
                        </button>
                      )}
                      {fix.rest}
                      {because === "" ? null : " · "}
                    </>
                  )}
                  {because}
                </p>
              )}
            </>
          )}
        </div>
        {/* Aged by the caller, as the box that used to carry it was. */}
        {elapsed === undefined ? null : <span className="armada-lead__elapsed mono">{elapsed}</span>}
        {act}
      </div>
      {waiting}
    </div>
  );
}
