// What a Session's pull request panel holds beyond the attachment: the page Fleet reads from the
// forge when the panel opens, and the comment box's draft, kind and write. Apart from
// `session-pull-request.tsx`, which draws the panel.

import { useEffect, useRef, useState } from "react";
import { forgetKept, mayWrite } from "@armada/components";
import type { PullRequestComposerProps } from "@armada/components";
import type { PullRequestPage } from "@armada/protocol";
import type { PullRequestPageRead, SessionsDraft } from "@armada/screens/src/draft/sessions";

/** What the draft is kept under: the Session and the pull request it is about. */
const keepOf = (sessionId: string, number: number) => `pull-request:${sessionId}#${number}`;

/**
 * The page and the comment box for one Session's pull request. **Read when it opens and again
 * after every write answers**, which answers the fresh page. A page that could not be read leaves
 * the panel without tabs and without the box, and says why. A refused write says why and keeps the
 * draft; a write that went through clears it.
 */
export function usePullRequestPage(sessionId: string, number: number, draft: SessionsDraft | undefined) {
  const held = useRef(draft);
  held.current = draft;
  const [read, setRead] = useState<PullRequestPageRead | undefined>(undefined);
  const [kind, setKind] = useState<PullRequestComposerProps["kind"]>("comment");
  const [text, setText] = useState("");
  const [pending, setPending] = useState(false);
  const [said, setSaid] = useState<string | undefined>(undefined);
  const here = useRef(true);
  const served = draft?.pullRequestPage !== undefined;

  useEffect(() => {
    here.current = true;
    return () => {
      here.current = false;
    };
  }, []);

  useEffect(() => {
    if (!served) return;
    let current = true;
    void held.current?.pullRequestPage?.(sessionId, number).then((answer) => {
      if (current && here.current) setRead(answer);
    });
    return () => {
      current = false;
    };
  }, [served, sessionId, number]);

  const send = () => {
    const write = held.current?.writePullRequest;
    if (write === undefined || pending || !mayWrite(kind, text)) return;
    const body = text.trim();
    setPending(true);
    setSaid(undefined);
    void write(sessionId, number, { kind, ...(body === "" ? {} : { body }) }).then((answer) => {
      if (answer.ok) forgetKept(keepOf(sessionId, number));
      if (!here.current) return;
      setPending(false);
      if (answer.ok) {
        setRead(answer);
        setText("");
        setKind("comment");
      } else {
        setSaid(answer.said);
      }
    });
  };

  const page: PullRequestPage | undefined = read?.ok === true ? read.page : undefined;
  const composer: PullRequestComposerProps | undefined =
    page !== undefined && draft?.writePullRequest !== undefined
      ? { kind, onKind: setKind, value: text, onChange: setText, keep: keepOf(sessionId, number), pending, ...(said === undefined ? {} : { said }), onSend: send }
      : undefined;
  return { page, pageSaid: read?.ok === false ? read.said : undefined, composer };
}
