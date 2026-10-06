#!/usr/bin/env python3
"""Reads `cargo xtask verify-foundations` as a delta against main's own run.

The reading is the merge line's (docs/capabilities/merge-line.md, *How a
verify-foundations run is read*, and crates/armada/src/land/gate.rs), which
this file follows rule for rule: only FAIL and `missing:` lines count, line
numbers are normalised out of a subject, findings are counted rather than
collected into a set, and a non-zero exit that names no rule is red.

    foundations_delta.py report OUT EXIT          exit 0 when the run named its rules
    foundations_delta.py delta BASE_OUT RUN_OUT RUN_EXIT   markdown on stdout, exit 1 when red
"""
import re
import sys
from collections import Counter


def failing_lines(text):
    return [
        line
        for line in (raw.strip() for raw in text.splitlines())
        if line.startswith("FAIL") or line.startswith("missing:")
    ]


def strip_line_numbers(line):
    return re.sub(r":\d+", "", line)


def a_report(text, exit_code):
    # A branch that breaks xtask prints one compile error and no rule.
    return exit_code == 0 or bool(failing_lines(text))


def new_lines(base_text, run_text):
    known = Counter(strip_line_numbers(line) for line in failing_lines(base_text))
    seen = Counter()
    new = []
    for line in failing_lines(run_text):
        subject = strip_line_numbers(line)
        seen[subject] += 1
        if seen[subject] > known[subject]:
            new.append(line)
    return new


def delta(base_text, run_text, run_exit):
    """(red, markdown)."""
    if not a_report(run_text, run_exit):
        tail = "\n".join(run_text.strip().splitlines()[-10:])
        return True, (
            "### foundations: red\n\nThe run exited %d and named no rule, so nothing "
            "was gated. Last lines:\n\n```\n%s\n```\n" % (run_exit, tail)
        )
    new = new_lines(base_text, run_text)
    known = failing_lines(base_text)
    out = ["### foundations: %s\n" % ("red" if new else "no new failing line")]
    if new:
        out.append("New against main's own run:\n\n```")
        out.extend(new)
        out.append("```\n")
    if known:
        out.append("<details><summary>Failing on main already, not counted against this change</summary>\n\n```")
        out.extend(known)
        out.append("```\n</details>\n")
    return bool(new), "\n".join(out)


def main(argv):
    read = lambda path: open(path, encoding="utf-8", errors="replace").read()
    if len(argv) == 4 and argv[1] == "report":
        return 0 if a_report(read(argv[2]), int(argv[3])) else 1
    if len(argv) == 5 and argv[1] == "delta":
        red, text = delta(read(argv[2]), read(argv[3]), int(argv[4]))
        print(text)
        return 1 if red else 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
