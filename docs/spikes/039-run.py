#!/usr/bin/env python3
"""Spike 39: drives 039-ask.swift and 039-readers.rs. One call at a time.

  039-run.py fixtures            the invented turns, through the account and the running summary,
                                 as shipped and with the prompts' own examples removed
  039-run.py snaps <dir>         real turns from Claude Code transcripts (kept outside the repository):
                                 extract, then run the account on each
  039-run.py proposer            the proposer on the invented requests

Build first (paths below are where this spike put them):
  swiftc -O -parse-as-library -target arm64-apple-macosx26.4 039-ask.swift -o $FM/ask
  rustc --edition 2021 -O 039-readers.rs -o $FM/readers     # mounts crates/fleet/src/session_account.rs unchanged
"""
import json, os, re, subprocess, sys, glob, random

FM = os.environ.get('FM', '/tmp/fm-spike')
ASK, READERS, W = f'{FM}/ask', f'{FM}/readers', f'{FM}/work'
os.makedirs(W, exist_ok=True)
RS = '\x1e'
CAP = ('--max', '200')  # response-token cap: without one a call ran 56 s repeating itself


def w(name, text):
    path = f'{W}/{name}'
    open(path, 'w').write(text)
    return path


def ask(question, flags=CAP):
    """One call, one process, measured with /usr/bin/time -l."""
    timing = f'{W}/time.txt'
    with open(w('q.txt', question)) as fin:
        r = subprocess.run(['/usr/bin/time', '-l', '-o', timing, ASK, *flags], stdin=fin, capture_output=True, text=True)
    t = open(timing).read()

    def g(pat):
        m = re.search(pat, t)
        return float(m.group(1)) if m else None
    try:
        meta = json.loads(r.stderr.strip().splitlines()[-1])
    except Exception:
        meta = {'error': 'no-meta: ' + r.stderr[-200:]}
    meta.update(answer=r.stdout.strip(), cpu_s=(g(r'([\d.]+)\s+user') or 0) + (g(r'([\d.]+)\s+sys') or 0),
                real_s=g(r'([\d.]+)\s+real'), rss_mb=(g(r'(\d+)\s+maximum resident set size') or 0) / 1e6)
    return meta


def readers(*a):
    return subprocess.run([READERS, *a], capture_output=True, text=True).stdout


def account_q(s):
    return readers('q-account', w('m.txt', s['message']), w('c.txt', RS.join(s['calls'])),
                   w('s.txt', RS.join(x.replace('\n', ' ') for x in s['said'])))


def sofar_q(s, done, last=''):
    return readers('q-sofar', w('l.txt', last), w('m.txt', s['message']), w('d.txt', RS.join(done)),
                   w('s.txt', RS.join(x.replace('\n', ' ') for x in s['said'])))


EXAMPLE_A = ' Say what the work is for instead: "the sign-in page", never "login.tsx".'
EXAMPLE_B = ' State facts in the past tense with no subject: "Found the reads happen once per Job. Added a cache, not keyed yet." Say what changed'


def without_examples(q):
    return q.replace(EXAMPLE_A, ' Say what the work is for instead.').replace(
        EXAMPLE_B, ' State facts in the past tense with no subject. Say what changed')


# ---- the invented turns: shaped like the real ones, naming nothing of any repository ----
FIXTURES = [
    {'id': 'f1', 'message': 'make the profile page remember my last tab',
     'calls': ['Reading ProfilePage.tsx', 'Editing ProfilePage.tsx', 'Running pnpm test'],
     'said': ['Added a stored choice for the last tab and read it back when the page opens. Running the tests.']},
    {'id': 'f2', 'message': 'the export button opens a new tab, make it download the file instead',
     'calls': ['Reading ExportButton.tsx', 'Editing ExportButton.tsx', 'Running git diff'],
     'said': ['The button was a link that opens a new tab; it is now a download link.']},
    {'id': 'f3', 'message': 'Merge main into your branch before your next push; it is 12 commits behind.',
     'calls': ['Running git fetch', 'Running git merge', 'Running pnpm test'],
     'said': ['Merged main with no conflicts. The tests are running.']},
    {'id': 'f4', 'message': 'why is the nightly sync taking four hours now?',
     'calls': ['Searching', 'Reading queue.rs', 'Reading retry.rs', 'Running cargo test'],
     'said': ['The retry loop waits a full minute between tries, even for a failure that cannot succeed. Checking the retry tests.']},
    {'id': 'f5', 'message': 'Approved: http://localhost:5200/?walk=invoice-list',
     'calls': ['Running git add', 'Running git push', 'Running gh pr'],
     'said': ['The pull request is open and CI is running.']},
    {'id': 'f6', 'message': 'fix the price rounding',
     'calls': ['Reading money.rs', 'Editing money.rs'],
     'said': ['Changed the total: -    let t = (a * 100.0).floor() as i64;\n+    let t = (a * 100.0).round() as i64;\n so a cent is no longer lost.']},
]
WORKFLOWS = None  # filled by the proposer subcommand from `armada job list_workflows`

PROPOSER_FIXTURES = [
    ('The sign-in page shows a blank screen when the password has an apostrophe in it. Find out why and fix it.', 'bug'),
    ('Add a dark mode toggle to the settings page and remember the choice between visits.', 'feature'),
    ('Rename the Account type to Profile everywhere and move it into its own module, no behaviour change.', 'refactor'),
    ('Undo the change that made the export button open a new tab; it broke the shortcut.', 'revert'),
    ('Review the open pull request for the invoice rounding change and tell me if it is safe to merge.', 'code_review'),
    ('Try whether a local cache in front of the price lookup would cut the page load time. A rough spike is enough.', 'prototype'),
    ('Write a design for how notifications should be grouped when ten arrive at once. I want to read it before anything is built.', 'design_plan'),
    ('Check the onboarding doc against what the app does today and list what is out of date, then fix the doc.', 'doc_review'),
    ('Run the whole migration: split the report generator into three smaller jobs, dispatch each one, and keep an eye on them until they are all in.', 'orchestrate'),
    ('Fix the typo in the footer: "recieve" should be "receive".', 'bug'),
    ('What time zone does the scheduler use? Just answer, do not change anything.', 'none'),
    ('Make the table on the orders page sortable by date and by total, and add a test for each.', 'feature'),
    ('Why is the nightly sync taking four hours now? Find the cause and fix it, across the queue, the store and the retry code.', 'bug'),
    ('Pull the retry logic out of the uploader and the downloader into one shared helper.', 'refactor'),
]


def fixtures():
    for s in FIXTURES:
        for name, make in (('account', account_q), ('account-noexample', lambda s: without_examples(account_q(s)))):
            r = ask(make(s))
            r['read'] = readers('r-account', w('a.txt', r['answer'])).strip().splitlines()[0]
            print(json.dumps({'id': s['id'], 'prompt': name, 'first_ms': r.get('first_ms'), 'total_ms': r.get('total_ms'),
                              'reader': r['read'], 'answer': r['answer']}))
        done = [x[6:] for x in readers('r-account', w('a.txt', ask(account_q(s))['answer'])).splitlines() if x.startswith('done: ')]
        for name, make in (('sofar', sofar_q), ('sofar-noexample', lambda s, d: without_examples(sofar_q(s, d)))):
            r = ask(make(s, done))
            r['read'] = readers('r-sofar', w('a.txt', r['answer'])).strip().splitlines()[0]
            print(json.dumps({'id': s['id'], 'prompt': name, 'first_ms': r.get('first_ms'), 'total_ms': r.get('total_ms'),
                              'reader': r['read'], 'answer': r['answer']}))


# ---- the proposer: SESSION_ANSWER_FORMAT as session_proposer.rs ships it, and its readers ported ----
def proposer():
    ids = ['bug', 'code_review', 'design_plan', 'doc_review', 'feature', 'orchestrate', 'prototype', 'refactor', 'revert']
    steps = {'bug': 'Find the cause -> Fix -> Tests -> Deliver', 'code_review': 'Read the diff -> Assess -> Deliver the review',
             'design_plan': 'Draft -> Present', 'doc_review': 'Compare -> Settle mismatches -> Plan the edit -> Draft -> Publish',
             'feature': 'Propose -> Walk -> Refine -> Deliver', 'orchestrate': 'Assess -> Plan -> Dispatch -> Oversee -> Summarise',
             'prototype': 'Frame -> Build -> Write up', 'refactor': 'Scope the refactor -> Restructure -> Review the change',
             'revert': 'Undo the change -> Summarise'}
    models, efforts = ['opus', 'sonnet', 'haiku'], {'low', 'medium', 'high', 'xhigh', 'max'}
    held = '\n'.join(f'  {i} — {i}\n    {steps[i]}' for i in ids)
    fmt = ('Answer with nothing but the block below.\n\n    workflow: <the id, spelled exactly as it appears above, or `none`>\n'
           '    title: <what to call this Job, in a few of the words the request used>\n'
           '    settings: model=<one of the models listed above, spelled exactly>, effort=<low, medium or high>\n\n'
           'Write the lines in the order above.\n\nAnswer `workflow: none` where no workflow above fits the request. A Job with no workflow works without steps laid out for it, and a near miss would hold it to steps that are not its work.\n\n'
           'Pick the model and the effort by how hard the work reads. A small change to a file the request names wants the lightest model and the lowest effort; work across many files, a design or a hard diagnosis wants the strongest model and a higher effort. Leave a pair out where you cannot tell.')

    def field(ans, name):
        for line in ans.splitlines():
            l = line.strip()
            if l.startswith(name + ':') and l[len(name) + 1:].strip():
                return l[len(name) + 1:].strip()

    def setting(ans, key):
        for pair in (field(ans, 'settings') or '').split(','):
            if '=' in pair and pair.split('=', 1)[0].strip().lower() == key:
                return pair.split('=', 1)[1].strip() or None
    for request, expect in PROPOSER_FIXTURES:
        q = ('You are deciding what kind of work a person just asked an agent to do, so that the agent starts on the right footing. Answer only the question at the end.\n\n'
             f'The request, as the person wrote it:\n\n{request}\n\nThe workflows this repository holds, and the steps each runs:\n\n{held}\n\n'
             'The models this machine can run the agent on:\n\n' + '\n'.join(f'  {m}' for m in models) + f'\n\n{fmt}')
        r = ask(q)
        named = field(r['answer'], 'workflow')
        print(json.dumps({'expect': expect, 'workflow': named, 'held': named in ids, 'model_ok': setting(r['answer'], 'model') in models,
                          'effort_ok': (setting(r['answer'], 'effort') or '').lower() in efforts, 'title': field(r['answer'], 'title'),
                          'first_ms': r.get('first_ms'), 'total_ms': r.get('total_ms')}))


if __name__ == '__main__':
    {'fixtures': fixtures, 'proposer': proposer}[sys.argv[1]]()
