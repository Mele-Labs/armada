// The repository's open issues, beside the composer, each one a dispatch away. #91,
// `docs/concepts/tracker.md`.
//
// **A view and nothing more** — the owner's call of 11 Oct 2026: nothing here becomes a Job until he
// presses Dispatch, and that press is the composer's own request with the issue's address as its
// words, so Fleet reads the issue exactly as it reads a pasted link.

import { useCallback, useEffect, useState } from "react";
import type { IssueList } from "@armada/protocol";
import {
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Stamp,
  Table,
  TableBody,
  TableCell,
  TableRow,
} from "@armada/components";
import { said } from "@armada/screens/src/copy";

type Read = { state: "reading" } | { state: "read"; list: IssueList } | { state: "failed"; why: string };

/** Asked of Fleet, never of the forge: Bridge talks to Fleet and nothing else. */
async function read(manifestId: string, refresh: boolean): Promise<Read> {
  const answer = await window.armada.listIssues(manifestId, refresh);
  return answer.ok ? { state: "read", list: answer.value } : { state: "failed", why: said(answer.outcome) };
}

export function OpenIssues({
  manifestId,
  disabled,
  onDispatch,
}: {
  /** The repository's Manifest: the list is that repository's. */
  manifestId: string;
  disabled: boolean;
  /** The composer's own press, with the issue's address as the request. */
  onDispatch: (url: string) => void;
}) {
  const [shown, setShown] = useState<Read>({ state: "reading" });
  const load = useCallback(
    (refresh: boolean) => {
      let current = true;
      void read(manifestId, refresh).then((next) => {
        if (current) setShown(next);
      });
      return () => {
        current = false;
      };
    },
    [manifestId],
  );
  useEffect(() => load(false), [load]);
  // Nothing to choose from is not worth a card on the composer.
  if (shown.state === "read" && shown.list.issues.length === 0 && shown.list.stale === undefined) return null;
  return (
    <Card aria-label="Open issues">
      <CardHeader>
        <CardTitle>Open issues</CardTitle>
        <CardDescription>
          {shown.state === "failed"
            ? `The issues could not be read. ${shown.why}`
            : shown.state === "read" && shown.list.stale !== undefined
              ? <>This is the list from <Stamp at={shown.list.fetched_at} />. {shown.list.stale}</>
              : "Dispatch one and Fleet reads the issue as the request. Its pull request closes the issue when it merges."}
        </CardDescription>
        <Button size="sm" variant="ghost" onClick={() => void load(true)} disabled={shown.state === "reading"}>
          Refresh
        </Button>
      </CardHeader>
      {shown.state !== "read" ? null : (
        <CardContent>
          <Table>
            <TableBody>
              {shown.list.issues.map((issue) => (
                <TableRow key={issue.url}>
                  <TableCell variant="mono">{issue.reference}</TableCell>
                  <TableCell>{issue.title}</TableCell>
                  <TableCell variant="metadata">
                    {issue.linked_pull_request ? "A pull request names it" : issue.labels.join(", ")}
                  </TableCell>
                  <TableCell>
                    <Button
                      size="sm"
                      variant="secondary"
                      ground="card"
                      aria-label={`Dispatch issue ${issue.reference}`}
                      disabled={disabled}
                      onClick={() => onDispatch(issue.url)}
                    >
                      Dispatch
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      )}
    </Card>
  );
}
