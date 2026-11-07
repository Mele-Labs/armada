#!/usr/bin/env python3
"""The person's acts, against the dev Fleet's own routes. `python3 -I person.py --help`.

Does what the owner would while a Job runs: approve the plan the Job recorded, approve a
sketch or a document, answer a question card, allow or refuse a permission, and send the
Job on when it asks in a reply. Every act is appended to a JSON lines file so the
timeline can show it.

  person.py watch --scratch DIR --session ID --acts FILE [--max-minutes 40] [--quiet-seconds 420]

`watch` polls GET /sessions/one every 3 s and exits with one line saying why:
  done          the Job called all_done, or its row ended
  timeout       --max-minutes passed (the run is stopped by the caller)
  quiet         nothing running, nothing waiting and no change for --quiet-seconds

The acts and the routes they use:

  plan item         POST /sessions/plan/approve        {session_id, version}
  sketch item       POST /sessions/sketch/answer       {session_id, sketch, drawn, how: approved}
  document item     POST /sessions/document/answer     {session_id, document, revision, how: approved}
  question card     POST /sessions/waiting/answer      {session_id, item_id, choice}
  permission        POST /sessions/waiting/answer      {session_id, item_id, choice}  (allow, unless unsafe)
  agent item        POST /sessions/waiting/answer      {session_id, item_id, choice | text}
  delivery item     POST /sessions/waiting/answer      {session_id, item_id: "delivery", choice: 2}   (Keep local)
  any other item with options   choice 0, logged as answer_unlisted_<source>
  walk item         POST /sessions/waiting/answer      {session_id, item_id, choice: 0}   (not exercised yet)
  reply that asks   POST /sessions/message             {session_id, text}
"""
import argparse
import json
import re
import sys
import time
import urllib.error
import urllib.request

UNSAFE = re.compile(
    r"(\bgh\b|\bbd\b|\bcurl\b|\bwget\b|\bssh\b|\bsudo\b|rm -rf|--force|push[^\n]*\bmain\b|/Users/[^/ ]+/(?!Library/Application Support/Armada/attachments/job-scratch))",
)


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + ".%03dZ" % (int(time.time() * 1000) % 1000)


class Fleet:
    def __init__(self, scratch):
        runtime = f"{scratch}/home/user/Library/Application Support/Armada/fleet.json"
        self.base = "http://127.0.0.1:%d" % json.load(open(runtime))["port"]

    def get(self, path):
        with urllib.request.urlopen(self.base + path, timeout=30) as r:
            return json.load(r)

    def post(self, path, body):
        req = urllib.request.Request(
            self.base + path, json.dumps(body).encode(), {"content-type": "application/json"}
        )
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                return r.status, json.loads(r.read() or b"null")
        except urllib.error.HTTPError as e:
            return e.code, e.read().decode(errors="replace")[:300]


def option_index(options, wanted, unwanted=None):
    for i, o in enumerate(options):
        label = (o.get("label") if isinstance(o, dict) else str(o)).lower()
        if re.search(wanted, label) and not (unwanted and re.search(unwanted, label)):
            return i
    return None


def act_on(fleet, sid, item, session, said, log):
    """One item, one act. Returns the act's name, or None when this item is not the person's to answer."""
    source, iid, text = item.get("source"), item["id"], item.get("text", "")
    options = item.get("options") or []
    attached = {(a["kind"], a["target"]): a for a in session.get("attachments", [])}

    def done(name, status, answer):
        log(name, iid, status, text[:100], answer)
        return name

    if source == "plan":
        version = int(iid.split(":", 1)[1])
        status, body = fleet.post("/sessions/plan/approve", {"session_id": sid, "version": version})
        return done("approve_plan", status, body if status >= 300 else "")
    if source == "sketch":
        a = attached.get(("sketch", iid)) or {}
        status, body = fleet.post(
            "/sessions/sketch/answer",
            {"session_id": sid, "sketch": iid, "drawn": a.get("detail", {}).get("drawn", ""), "how": "approved"},
        )
        return done("approve_sketch", status, body if status >= 300 else "")
    if source == "document":
        a = attached.get(("document", iid)) or {}
        status, body = fleet.post(
            "/sessions/document/answer",
            {"session_id": sid, "document": iid, "revision": a.get("detail", {}).get("revision", "1"), "how": "approved"},
        )
        return done("approve_document", status, body if status >= 300 else "")
    if source in ("permission", "ask_card"):
        unsafe = bool(UNSAFE.search(text + " " + json.dumps(item.get("detail", ""))))
        if unsafe:
            choice = option_index(options, r"refuse|deny|block|no\b", r"allow")
            name = "refuse_permission"
        else:
            choice = option_index(options, r"allow.*once|^allow\b|^yes|approve")
            name = "allow_permission" if source == "permission" else "answer_question"
        choice = 0 if choice is None else choice
        status, body = fleet.post("/sessions/waiting/answer", {"session_id": sid, "item_id": iid, "choice": choice})
        return done(name, status, body if status >= 300 else "")
    if source == "walk":
        status, body = fleet.post("/sessions/waiting/answer", {"session_id": sid, "item_id": iid, "choice": 0})
        return done("approve_walk", status, body if status >= 300 else "")
    if source == "delivery":
        # No forge here: "Keep local" (2) is the only choice that does not name a pull request.
        status, body = fleet.post("/sessions/waiting/answer", {"session_id": sid, "item_id": iid, "choice": 2})
        return done("answer_delivery_keep_local", status, body if status >= 300 else "")
    if source == "agent":
        if options:
            status, body = fleet.post("/sessions/waiting/answer", {"session_id": sid, "item_id": iid, "choice": 0})
        else:
            status, body = fleet.post(
                "/sessions/waiting/answer",
                {"session_id": sid, "item_id": iid, "text": "Go ahead with your best judgement."},
            )
        return done("answer_agent_item", status, body if status >= 300 else "")
    if source == "replied" and said.get(iid, 0) < 1:
        said[iid] = said.get(iid, 0) + 1
        status, body = fleet.post(
            "/sessions/message", {"session_id": sid, "text": "Go ahead with your best judgement."}
        )
        return done("message_go_ahead", status, body if status >= 300 else "")
    if options and source not in ("done", "replied", "first_message"):
        status, body = fleet.post("/sessions/waiting/answer", {"session_id": sid, "item_id": iid, "choice": 0})
        return done(f"answer_unlisted_{source}_choice_0", status, body if status >= 300 else "")
    return None


def watch(args):
    fleet = Fleet(args.scratch)
    sid = args.session
    started = time.time()
    acted, said = set(), {}
    last_change, last_fingerprint = time.time(), None

    def log(name, item, status, text, answer):
        with open(args.acts, "a") as f:
            f.write(json.dumps({"at": now(), "act": name, "item": item, "status": status, "text": text, "answer": answer}) + "\n")

    while True:
        try:
            one = fleet.get(f"/sessions/one?session_id={sid}")
        except Exception as e:  # Fleet busy; try again
            print("poll failed:", e, file=sys.stderr)
            time.sleep(3)
            continue
        session = one["session"]
        items = session.get("waiting_for") or []
        running = (session.get("hosted") or {}).get("turn", {}).get("state") not in (None, "idle")
        if session.get("done_since") or session.get("state") != "live":
            print("done")
            return 0
        for item in items:
            key = (item["id"], item.get("since"))
            if item.get("source") == "done" or key in acted:
                continue
            if act_on(fleet, sid, item, session, said, log):
                acted.add(key)
        fingerprint = json.dumps([len(one.get("rows", [])), [i["id"] for i in items], running])
        if fingerprint != last_fingerprint:
            last_fingerprint, last_change = fingerprint, time.time()
        if time.time() - started > args.max_minutes * 60:
            print("timeout")
            return 0
        if not running and time.time() - last_change > args.quiet_seconds:
            print("quiet")
            return 0
        time.sleep(3)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    w = sub.add_parser("watch")
    w.add_argument("--scratch", required=True)
    w.add_argument("--session", required=True)
    w.add_argument("--acts", required=True)
    w.add_argument("--max-minutes", type=float, default=40)
    w.add_argument("--quiet-seconds", type=float, default=420)
    args = ap.parse_args()
    sys.exit(watch(args))


if __name__ == "__main__":
    main()
