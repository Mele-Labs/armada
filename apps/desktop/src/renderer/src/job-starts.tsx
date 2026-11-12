// Settings › Job starts: per project, whether a Job that another Job or the Operator starts runs at once or waits on its start card for
// him (`values.agent_starts`). His own starts always run at once, so the pane is about the rest. Unset is Start at once.

import { useEffect, useState } from "react";
import { CircleHelp, Play } from "lucide-react";
import { Select, Tooltip } from "@armada/components";
import type { Project, ValuesSet } from "@armada/protocol";

import { refusal } from "./annotate/send";
import "./job-starts.css";

type AgentStarts = "at_once" | "ask_me";

const CHOICES: Readonly<Record<AgentStarts, string>> = {
  at_once: "Start at once",
  ask_me: "Ask me",
};

/** What a project sets, `at_once` where it sets nothing. */
function startsOf(project: Project): AgentStarts {
  const set = (project.values as (ValuesSet & { agent_starts?: string }) | undefined)?.agent_starts;
  return set === "ask_me" ? "ask_me" : "at_once";
}

export function JobStarts({ live }: { live: boolean }) {
  const [projects, setProjects] = useState<readonly Project[]>();
  const [refused, setRefused] = useState<string>();
  useEffect(() => {
    void window.armada.listProjects().then((read) => {
      if (read.ok) setProjects(read.value.projects.filter((one) => one.unread === undefined));
    });
  }, []);

  const choose = (project: Project, next: AgentStarts) => {
    const was = projects;
    // Shown at the press; a refusal puts it back and says why.
    setProjects(
      (now) =>
        now?.map((one) =>
          one.id === project.id
            ? {
                ...one,
                values: {
                  ...one.values,
                  agent_starts: next,
                } as ValuesSet,
              }
            : one,
        ) ?? now,
    );
    setRefused(undefined);
    void window.armada
      .editProject(project.id, {
        values: {
          ...project.values,
          agent_starts: next,
        } as ValuesSet,
      })
      .then((acted) => {
        if (acted.ok) return;
        setProjects(was);
        setRefused(refusal(acted.outcome));
      });
  };

  return (
    <section className="armada-job-starts" aria-label="Jobs started for you">
      <h4 className="armada-job-starts__title" aria-hidden="true">
        Jobs started for you
      </h4>
      <p className="armada-job-starts__help">
        Jobs another Job or the Operator starts in a project. Ask me holds each on its start card until you press Start. Jobs you start run at
        once.
      </p>
      {refused === undefined ? null : <p className="armada-job-starts__refused">{refused}</p>}
      {projects === undefined ? (
        <div className="armada-job-starts__skeleton" aria-hidden="true" />
      ) : (
        <ul className="armada-job-starts__rows">
          {projects.map((one) => {
            const starts = startsOf(one);
            const Glyph = starts === "at_once" ? Play : CircleHelp;
            return (
              <li key={one.id} className="armada-job-starts__row" data-starts={starts}>
                <Tooltip label={CHOICES[starts]}>
                  <span className="armada-job-starts__glyph" role="img" aria-label={CHOICES[starts]}>
                    <Glyph aria-hidden />
                  </span>
                </Tooltip>
                <span className="armada-job-starts__name">{one.name}</span>
                <span className="armada-job-starts__choice">
                  <Select
                    aria-label={`Jobs started for you in ${one.name}`}
                    value={starts}
                    disabled={!live}
                    onChange={(event) => choose(one, event.target.value as AgentStarts)}
                  >
                    {(Object.keys(CHOICES) as AgentStarts[]).map((key) => (
                      <option key={key} value={key}>
                        {CHOICES[key]}
                      </option>
                    ))}
                  </Select>
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
