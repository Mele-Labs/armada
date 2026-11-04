// A Job named in a row, as one small chip: its live state's glyph and its title. A press opens it. The Projects tab's holder and the
// Operator's rows draw it, so a Job named in a line reads the same wherever it is.

import type { ButtonHTMLAttributes } from "react";
import { SessionMark, Tooltip } from "@armada/components";
import type { Session } from "@armada/screens/src/draft/sessions";

import { JobMark } from "./issues-tab/marks";
import { useSessions } from "./sessions-draft";
import "./job-chip.css";

/** One that has ended reads by the title it was given. `sessions` is the list the host holds, else the window's. `shortcut` is the host's key for the press. */
export function JobChip({
  job,
  sessions,
  onOpen,
  shortcut,
  className,
  ...rest
}: Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "onClick" | "type"> & {
  job: {
    id: string;
    title: string;
  };
  sessions?: readonly Session[];
  onOpen: (id: string) => void;
  shortcut?: string;
}) {
  const held = useSessions();
  const session = (sessions ?? held).find((one) => one.id === job.id);
  const title = session?.title ?? job.title;
  return (
    <Tooltip
      label="Open the Job"
      {...(shortcut === undefined
        ? {}
        : {
            shortcut,
          })}
      asChild
    >
      <button
        {...rest}
        type="button"
        className={className === undefined ? "job-chip" : `job-chip ${className}`}
        aria-label={`Open ${title}`}
        onClick={() => onOpen(job.id)}
      >
        {session === undefined ? (
          <SessionMark state="ended" said={`${title}: Ended`} />
        ) : (
          <JobMark session={session} />
        )}
        <span className="job-chip__title">{title}</span>
      </button>
    </Tooltip>
  );
}
