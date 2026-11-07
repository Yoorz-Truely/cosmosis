#!/usr/bin/env python3
"""Check a commit message: a subject under 120 characters, then a blank line before any body.

Usage: commit-msg.py <message file>   (lefthook's commit-msg hook calls it)
"""

import sys

lines = [x for x in open(sys.argv[1], encoding="utf-8").read().splitlines() if not x.startswith("#")]
problems = []
if not lines or not lines[0].strip():
    problems.append("the subject line is empty")
elif len(lines[0]) >= 120:
    problems.append(f"the subject line is {len(lines[0])} characters; keep it under 120")
if len(lines) > 1 and lines[1].strip():
    problems.append("leave a blank line after the subject")
for p in problems:
    print(f"commit-msg: {p}", file=sys.stderr)
sys.exit(1 if problems else 0)
