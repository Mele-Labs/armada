#!/usr/bin/env python3
"""The scripted person for spike 49. Every answer is scripted; none is judged live. `python3 -I person.py watch --help`.

Copy of 048-complex/person.py. Fixes from run 48: it never merges while any gate before Deliver is open (the owner would not),
it acts on the keeper's held items by rule, and its quiet-stop ignores the `done` item (run 48 counted it, so the rule never fired).

Rules and where each comes from: person.md in this folder. Acts, in the order a run reaches them:

  plan item           approve it (every version: the first, and the re-recorded one)
  document item       revision 1: send it back with the note "Say what this does not cover."; any later revision: approve
  after a push        once, when the keeper says the Job's own branch was pushed after the design note closed: send the change
                      request written in problem.md
  merge asked         merge armada/session-<id8> into origin's main ONLY when the keeper says merge_ready (every gate before
                      Deliver closed, branch tip is the commit Checks passed at). Otherwise answer what is missing
  held item           an item the keeper raised for a held gate (Approve / Skip this group). By rule: if the condition is met,
                      approve (plan: approve the plan; document: approve the open revision; merged: merge); if not, answer
                      what is missing in one sentence. Skip this group is never used
  question card       the first rule in rules.json that matches; else the option marked Recommended; else a sentence.
                      Never the first option by default. A question no rule met is logged as no_rule
  permission          allow unless the text names gh bd curl wget ssh sudo, rm -rf, --force, a push to main or the owner's home
  walk, sketch        approve
  delivery            Keep local (there is no forge)

Ends with one line: done (the Job called all_done with every gate closed), timeout, quiet, ended.
"""
import argparse
import json
import os
import re
import sqlite3
import subprocess
import sys
import time
import urllib.error
import urllib.request

UNSAFE = re.compile(
    r"(\bgh\b|\bbd\b|\bcurl\b|\bwget\b|\bssh\b|\bsudo\b|rm -rf|--force|push[^\n]*\bmain\b|/Users/[^/ ]+/(?!Library/Application Support/Armada/attachments/job-scratch))",
)
MERGE_ASKED = re.compile(r"(merge|merging)\b[^\n]{0,80}\b(branch|origin|main|armada/session|push)|armada/session-[0-9a-f]{8}[^\n]{0,80}merge|please merge", re.I)
NOTE = "Say what this does not cover."
CHANGE_REQUEST = (
    "One more thing: armada-mttk. A tab mod's card in a Job's thread titles it with `modTitle` in `mods-made.ts`, so `review-queue` reads "
    "Review-queue, while its tab reads Review queue through `tabTitle` in `job-panes/custom.ts`. Make the card read the tab's name."
)
FALLBACK = "Take the smaller change that never drops a layout of a Job that is still listed."


def now():
    return time.strftime("%Y-%m-%dT%H:%M:%S", time.gmtime()) + ".%03dZ" % (int(time.time() * 1000) % 1000)


class Fleet:
    def __init__(self, scratch):
        runtime = f"{scratch}/home/user/Library/Application Support/Armada/fleet.json"
        self.base = "http://127.0.0.1:%d" % json.load(open(runtime))["port"]
        self.db = f"{scratch}/home/user/Library/Application Support/Armada/armada.db"

    def get(self, path):
        with urllib.request.urlopen(self.base + path, timeout=30) as r:
            return json.load(r)

    def post(self, path, body):
        req = urllib.request.Request(self.base + path, json.dumps(body).encode(), {"content-type": "application/json"})
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                return r.status, json.loads(r.read() or b"null")
        except urllib.error.HTTPError as e:
            return e.code, e.read().decode(errors="replace")[:300]

    def document_revision(self, sid, target):
        """The newest revision of a document attachment, from the store: (revision, revisions kept, settled)."""
        con = sqlite3.connect(f"file:{self.db}?mode=ro", uri=True)
        try:
            best = None
            for (detail,) in con.execute("select detail from ledger_attachment_versions where holder_id=? and kind='artifact' and target=? order by seq", (sid, target)):
                d = json.loads(detail)
                if isinstance(d, dict) and d.get("form") == "document":
                    best = d
            if not best:
                return None
            return best.get("revision"), len(json.loads(best.get("revisions", "[]"))), best.get("settled")
        finally:
            con.close()

    def open_document(self, sid):
        """The newest unsettled page under .armada/documents: its target, or None."""
        con = sqlite3.connect(f"file:{self.db}?mode=ro", uri=True)
        try:
            best = {}
            for target, detail in con.execute("select target, detail from ledger_attachment_versions where holder_id=? and kind='artifact' order by seq", (sid,)):
                d = json.loads(detail)
                if isinstance(d, dict) and d.get("form") == "document" and re.search(r"/\.armada/documents/[^/]+\.html$", target):
                    best[target] = d
            open_ = [t for t, d in best.items() if d.get("settled") != "approved"]
            return open_[-1] if open_ else None
        finally:
            con.close()


def merge_branch(scratch, sid):
    """The person's merge, done in a scratch clone of the bare origin: the Job's branch into origin's main."""
    origin, id8 = f"{scratch}/origin.git", sid[:8]
    branch = f"armada/session-{id8}"
    work = f"{scratch}/merge-049-{id8}"

    def git(*args):
        r = subprocess.run(["git", "-c", "user.name=The person", "-c", "user.email=person@example.invalid", *args], cwd=work if os.path.isdir(work) else None, capture_output=True, text=True)
        return r.returncode, (r.stdout + r.stderr).strip()

    if not os.path.isdir(work):
        subprocess.run(["git", "clone", "--quiet", "--local", origin, work], capture_output=True)
    git("fetch", "--quiet", "origin")
    rc, _ = git("rev-parse", "--verify", "-q", f"origin/{branch}")
    if rc != 0:
        return None
    git("checkout", "--quiet", "-B", "main", "origin/main")
    rc, out = git("merge", "--no-ff", "-m", f"Merge {branch}", f"origin/{branch}")
    if rc != 0:
        return None
    rc, out = git("push", "--quiet", "origin", "HEAD:refs/heads/main")
    return f"Merged {branch} into main." if rc == 0 else None


def label_of(o):
    if isinstance(o, dict):
        return f"{o.get('label', '')} {o.get('description', '')}".lower()
    return str(o).lower()


def option_index(options, wanted, unwanted=None):
    for i, o in enumerate(options):
        label = label_of(o)
        if re.search(wanted, label) and not (unwanted and re.search(unwanted, label)):
            return i
    return None


def watch(args):
    fleet = Fleet(args.scratch)
    sid = args.session
    rules = json.load(open(args.rules))
    started = time.time()
    acted = set()
    acted_holds = set()
    flags = {"note_sent": False, "change_sent": False, "merged": False}
    told_missing = {}  # (kind, missing text) -> time, so one missing fact is told once
    last_change, last_fingerprint = time.time(), None
    done_at = None
    if args.resume and os.path.exists(args.acts):  # a restart of this script in the same run keeps what it already did
        for l in open(args.acts):
            d = json.loads(l)
            if d["act"] == "document_send_back" and d["status"] < 300:
                flags["note_sent"] = True
            if d["act"] == "change_request" and d["status"] < 300:
                flags["change_sent"] = True
            if d["act"] == "merge_branch":
                flags["merged"] = True
            if d["act"].startswith("held_item"):
                acted_holds.add(d["item"])

    def log(name, item, status, text, answer, why=""):
        with open(args.acts, "a") as f:
            f.write(json.dumps({"at": now(), "act": name, "item": item, "status": status, "text": text[:160], "answer": answer, "why": why}) + "\n")

    def keeper_state():
        try:
            return json.load(open(f"{args.run_dir}/state.json"))
        except (OSError, ValueError):
            return {}

    def answer_item(item, name, why, choice=None, text=None):
        body = {"session_id": sid, "item_id": item["id"]}
        if choice is not None:
            body["choice"] = choice
        else:
            body["text"] = text
        status, resp = fleet.post("/sessions/waiting/answer", body)
        log(name, item["id"], status, item.get("text", ""), (choice if choice is not None else text), why)
        return status < 300

    def say(name, item_id, text, why):
        status, _ = fleet.post("/sessions/message", {"session_id": sid, "text": text})
        log(name, item_id, status, "", text, why)
        return status < 300

    def document_act(iid, text):
        target = iid[len("document:"):] if iid.startswith("document:") else iid
        rev = fleet.document_revision(sid, target)
        if rev is None:
            return False
        revision, kept, settled = rev
        if settled:
            return True
        if not flags["note_sent"]:
            status, resp = fleet.post("/sessions/document/answer", {"session_id": sid, "document": iid, "revision": revision, "how": "sent_back", "words": NOTE})
            log("document_send_back", iid, status, text, NOTE, f"scripted: the first revision gets one note; revision {revision} of {kept}")
            if status < 300:
                flags["note_sent"] = True
            return status < 300
        status, resp = fleet.post("/sessions/document/answer", {"session_id": sid, "document": iid, "revision": revision, "how": "approved"})
        log("document_approve", iid, status, text, "", f"scripted: approve the revision after the note; revision {revision} of {kept}")
        return status < 300

    def merge_now(item_id, text, source_name):
        said_ = merge_branch(args.scratch, sid)
        if said_:
            flags["merged"] = True
            log("merge_branch", item_id, 200, text, said_, source_name)
        return said_

    def merge_gate(st):
        """The person merges only when Deliver is live: every gate before it closed and the tip is what Checks passed at."""
        if st.get("merge_ready") and not st.get("gates_open_before_deliver"):
            return True, ""
        before = st.get("gates_open_before_deliver") or []
        return False, (f"still open before Deliver: {', '.join(before)}" if before else "the pushed commit is not the one Checks passed at")

    def act_hold(st):
        """The keeper's held items: by rule. Approve if the condition is met, otherwise answer what is missing."""
        for item in st.get("needs_you", []):
            if item["id"] in acted_holds:
                continue
            gate, miss = item["gate"], (st.get("missing") or {}).get(item["gate"], "")
            if gate == "plan_approved":
                status, _ = fleet.post("/sessions/plan/approve", {"session_id": sid, "version": st["plan_version"]})
                log("held_item_approve", item["id"], status, item["text"], f"plan v{st['plan_version']}", "scripted: the plan is there, approve it")
                acted_holds.add(item["id"])
            elif gate == "document_approved":
                target = fleet.open_document(sid) if st.get("doc_open") else None
                if target:
                    ok = document_act(f"document:{target}", item["text"])
                    log("held_item_approve", item["id"], 200 if ok else 500, item["text"], target, "scripted: a page is open, so act on it as on any document item")
                else:
                    say("held_item_answer", item["id"], f"Nothing for me to approve: {miss}. Write the design note as an HTML page under .armada/documents/ with the writing-a-document skill and show it.", "scripted: the condition (an open page) is not met")
                acted_holds.add(item["id"])
            elif gate == "merged" and flags["merged"]:
                acted_holds.add(item["id"])
            elif gate == "merged":
                ok, why = merge_gate(st)
                if ok:
                    said_ = merge_now(item["id"], item["text"], "scripted: held on Deliver and every earlier gate is closed")
                    log("held_item_approve", item["id"], 200 if said_ else 500, item["text"], said_ or "merge failed", "scripted: merge")
                else:
                    say("held_item_answer", item["id"], f"I have not merged: {miss or why}.", "scripted: Deliver is not live, so no merge")
                acted_holds.add(item["id"])
            else:
                say("held_item_answer", item["id"], f"Nothing for me to do on this one: {miss}.", "scripted: not a gate the person acts on")
                acted_holds.add(item["id"])

    def act_on(item):
        source, iid, text = item.get("source"), item["id"], item.get("text", "")
        options = item.get("options") or []
        detail = json.dumps(item.get("detail", ""))
        if source == "plan":
            status, resp = fleet.post("/sessions/plan/approve", {"session_id": sid, "version": int(iid.split(":", 1)[1])})
            log("approve_plan", iid, status, text, "", "scripted: approve every plan version")
            return status < 300
        if source == "document":
            return document_act(iid, text)
        if source == "sketch":
            status, resp = fleet.post("/sessions/sketch/answer", {"session_id": sid, "sketch": iid, "drawn": "", "how": "approved"})
            log("approve_sketch", iid, status, text, "", "scripted")
            return status < 300
        if source == "walk":
            return answer_item(item, "approve_walk", "scripted", choice=0)
        if source == "delivery":
            return answer_item(item, "answer_delivery_keep_local", "no forge here", choice=2)
        if source in ("permission",):
            unsafe = bool(UNSAFE.search(text + " " + detail))
            if unsafe:
                idx = option_index(options, r"refuse|deny|block|no\b", r"allow")
                return answer_item(item, "refuse_permission", "names an unsafe thing", choice=0 if idx is None else idx)
            idx = option_index(options, r"allow.*once|^allow\b|^yes|approve")
            return answer_item(item, "allow_permission", "not unsafe", choice=0 if idx is None else idx)
        if source in ("agent", "ask_card", "replied"):
            if MERGE_ASKED.search(text + " " + detail) and not flags["merged"]:
                st = keeper_state()
                ok, why = merge_gate(st)
                if not ok:
                    # asked too early: answer what is missing, every time it is asked
                    reply = f"I have not merged. Deliver is not live yet; {why}."
                    if source == "replied":
                        return say("merge_refused_gate_open", iid, reply, "scripted: never merge while a gate before Deliver is open")
                    return answer_item(item, "merge_refused_gate_open", "scripted: never merge while a gate before Deliver is open", text=reply)
                said_ = merge_now(iid, text, "scripted: merge when asked, Deliver live and every earlier gate closed")
                if said_:
                    if options:
                        idx = option_index(options, r"merg|yes|done|ok")
                        return answer_item(item, "answer_after_merge", "merged", choice=0 if idx is None else idx)
                    if source == "replied":
                        return say("answer_after_merge", iid, said_, "merged")
                    return answer_item(item, "answer_after_merge", "merged", text=said_)
            for rule in rules["rules"]:
                if re.search(rule["when"], text + " " + detail, re.I):
                    if options and rule.get("pick"):
                        idx = option_index(options, rule["pick"], rule.get("avoid"))
                        if idx is not None:
                            return answer_item(item, "answer_by_rule", rule["name"], choice=idx)
                    if rule.get("text"):
                        if source == "replied":
                            return say("answer_by_rule", iid, rule["text"], rule["name"])
                        return answer_item(item, "answer_by_rule", rule["name"], text=rule["text"])
            if source == "replied":
                return False  # a reply that only reports is not the person's to answer
            idx = option_index(options, r"recommended")
            if idx is not None:
                return answer_item(item, "answer_recommended", "no rule met; the option marked Recommended", choice=idx)
            return answer_item(item, "answer_no_rule_sentence", "no_rule: no rule met and no option marked Recommended", text=FALLBACK)
        if options and source not in ("done", "first_message"):
            return answer_item(item, f"answer_unlisted_{source}", "no_rule: an item kind the script does not know", choice=len(options) - 1)
        return False

    while True:
        try:
            one = fleet.get(f"/sessions/one?session_id={sid}")
        except Exception as e:
            print("poll failed:", e, file=sys.stderr)
            time.sleep(3)
            continue
        session = one["session"]
        items = [i for i in (session.get("waiting_for") or []) if i.get("source") != "done"]  # the `done` item is not a question
        running = (session.get("hosted") or {}).get("turn", {}).get("state") not in (None, "idle")
        if session.get("state") != "live":
            print("ended")
            return 0
        for item in items:
            key = (item["id"], item.get("since"))
            if key in acted:
                continue
            if act_on(item):
                acted.add(key)
        st = keeper_state()
        act_hold(st)
        if (
            not flags["change_sent"]
            and st.get("doc_closed_at")
            and st.get("own_pushes_after_doc", 0) >= 1
            and not any(i.get("source") == "plan" for i in items)
        ):
            status, _ = fleet.post("/sessions/message", {"session_id": sid, "text": CHANGE_REQUEST})
            log("change_request", "message", status, CHANGE_REQUEST, "", "scripted: one change request after the first push of the Job's branch")
            if status < 300:
                flags["change_sent"] = True
        fingerprint = json.dumps([len(one.get("rows", [])), [i["id"] for i in items], running, st.get("live"), [n["id"] for n in st.get("needs_you", [])]])
        if fingerprint != last_fingerprint:
            last_fingerprint, last_change = fingerprint, time.time()
        # the end: all_done with every gate closed, held for 20 s so a late notice is seen
        if session.get("done_since") and st.get("gates_open") == [] and not st.get("held"):
            done_at = done_at or time.time()
            if time.time() - done_at > 20 and not running:
                print("done")
                return 0
        else:
            done_at = None
        if time.time() - started > args.max_minutes * 60:
            print("timeout")
            return 0
        if not running and not items and time.time() - last_change > args.quiet_seconds:
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
    w.add_argument("--run-dir", required=True)
    w.add_argument("--rules", required=True)
    w.add_argument("--max-minutes", type=float, default=75)
    w.add_argument("--quiet-seconds", type=float, default=420)
    w.add_argument("--resume", action="store_true")
    sys.exit(watch(ap.parse_args()))


if __name__ == "__main__":
    main()
