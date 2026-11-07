#!/usr/bin/env python3
"""Commit code only after `just check` passed on that very tree (task 0-10).

`just check` calls `start <key>` first and `pass <key>` last. `pass` records the tree the check
ran on in `work/check/passed`, but only if the files did not change while it ran. The
commit-msg hook calls `require`: by then pre-commit's fixers have re-staged what they changed,
so the index is what will be committed. It refuses a commit that touches code unless its staged
tree has such a record. "The tree" is every file that is not ignored, as `git add -A` would
stage it, tracked or not; documents may differ from it, code may not. A merge Git made by itself
(MERGE_HEAD one commit, not HEAD or its ancestor, merging with no conflict, the staged code
exactly that merge's) and documents-only commits need no record; a conflict resolved by hand
needs `just check` like any commit.

Not code: `*.md` and everything under `docs/`; in Null and Cosmósis also images (`.png`, `.jpg`,
`.jpeg`, `.gif`, `.svg`, `.webp`). Sau builds its branding images into the app, so there they
are code.

Usage: check-record.py start <key> | pass <key> | require | selftest
The same file is in Sau (`sau/tools/`), Null and Cosmósis (`tools/`); Sau's check keeps the
copies equal. Docs: Sau's docs/agents/tooling.md ("The check record").
"""

import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import time

KEEP = 200  # records kept; older ones are dropped
CHECKED_AGAINST = 20  # the newest records a commit's code is compared with
IMAGES = (".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp")


def git(*args, cwd=None, env=None, check=True):
    r = subprocess.run(["git", *args], cwd=cwd, env=env, capture_output=True, text=True)
    if check and r.returncode:
        sys.exit(f"check-record: git {' '.join(args)} failed: {r.stderr.strip()}")
    return r.stdout


def top(cwd=None):
    return pathlib.Path(git("rev-parse", "--show-toplevel", cwd=cwd).strip())


def store(cwd=None):
    d = top(cwd) / "work" / "check"
    d.mkdir(parents=True, exist_ok=True)
    return d


def is_sau(root):
    return (root / "sau" / "tools" / "dev.py").is_file()


def is_code(path, sau):
    p = path.lower()
    if p.endswith(".md") or p.startswith("docs/"):
        return False
    return sau or not p.endswith(IMAGES)


def working_tree(cwd=None):
    """The tree of every file not ignored, as `git add -A` would stage it, without touching the
    real index: a copy of it takes the changes, so unchanged files are not hashed again."""
    index = pathlib.Path(git("rev-parse", "--path-format=absolute", "--git-path", "index", cwd=cwd).strip())
    with tempfile.TemporaryDirectory() as d:
        tmp = pathlib.Path(d) / "index"
        if index.exists():
            # copy2 keeps the index's mtime, so Git still re-reads files changed in the same
            # second as the index was written ("racily clean" entries); a fresh mtime would hide them.
            shutil.copy2(index, tmp)
        env = dict(os.environ, GIT_INDEX_FILE=str(tmp))
        git("add", "-A", ".", cwd=top(cwd), env=env)
        return git("write-tree", cwd=cwd, env=env).strip()


def start(key, cwd=None):
    (store(cwd) / f"started-{key}").write_text(working_tree(cwd) + "\n")
    return 0


def passed(key, cwd=None, quiet=False):
    s = store(cwd)
    marker = s / f"started-{key}"
    if not marker.exists():
        print("check-record: no start was recorded for this check; nothing recorded", file=sys.stderr)
        return 0
    began = marker.read_text().strip()
    marker.unlink()
    now = working_tree(cwd)
    if now != began:
        if not quiet:
            print(
                "check-record: files changed while the check ran, so this pass is not recorded; "
                "run `just check` again before committing code",
                file=sys.stderr,
            )
        return 0
    log = s / "passed"
    lines = log.read_text().splitlines() if log.exists() else []
    lines = [x for x in lines if not x.startswith(now + " ")][-(KEEP - 1) :]
    lines.append(f"{now} {time.strftime('%Y-%m-%dT%H:%M:%S')}")
    log.write_text("\n".join(lines) + "\n")
    if not quiet:
        print(f"check-record: passed on tree {now[:12]}")
    return 0


def is_clean_merge(git_dir, sau, cwd=None):
    """Whether this commit is a merge Git made by itself, which brings only commits checked on
    their own branches: MERGE_HEAD names one commit that is not HEAD or one of its ancestors,
    `git merge-tree` merges it into HEAD with no conflict, and the staged code is exactly that
    merge's (documents may differ). A MERGE_HEAD written by hand, a conflict resolved by hand
    or any other code changed in the merge is not exempt: it needs `just check` like any
    commit."""
    merge_head = git_dir / "MERGE_HEAD"
    if not merge_head.exists():
        return False
    heads = merge_head.read_text().split()
    if len(heads) != 1:
        return False  # an octopus merge is checked like any commit
    theirs = git("rev-parse", "--verify", "-q", heads[0] + "^{commit}", cwd=cwd, check=False).strip()
    if not theirs:
        return False
    ancestor = subprocess.run(["git", "merge-base", "--is-ancestor", theirs, "HEAD"], cwd=cwd, capture_output=True)
    if ancestor.returncode == 0:
        return False
    merged = subprocess.run(["git", "merge-tree", "--write-tree", "HEAD", theirs], cwd=cwd, capture_output=True, text=True)
    if merged.returncode != 0:
        return False  # a conflict: its resolution was written by hand
    result = merged.stdout.split()[0]
    staged = git("write-tree", cwd=cwd).strip()
    differ = [p for p in git("diff", "--name-only", "-z", result, staged, cwd=cwd).split("\0") if p]
    return not any(is_code(p, sau) for p in differ)


def require(cwd=None):
    root = top(cwd)
    git_dir = pathlib.Path(git("rev-parse", "--path-format=absolute", "--git-dir", cwd=cwd).strip())
    if is_clean_merge(git_dir, is_sau(root), cwd):
        return 0  # Git's own merge of commits checked on their own branches
    staged = [p for p in git("diff", "--cached", "--name-only", "-z", cwd=cwd).split("\0") if p]
    sau = is_sau(root)
    code = [p for p in staged if is_code(p, sau)]
    if not code:
        return 0
    tree = git("write-tree", cwd=cwd).strip()
    log = root / "work" / "check" / "passed"
    trees = [x.split()[0] for x in log.read_text().splitlines() if x.strip()] if log.exists() else []
    if tree in trees:
        return 0
    # The same code as a tree that passed, with only documents different (a report written
    # after the check, or one left for its own commit): the code is what was checked.
    for passed_tree in reversed(trees[-CHECKED_AGAINST:]):
        diff = git("diff", "--name-only", "-z", passed_tree, tree, cwd=cwd, check=False).split("\0")
        if not any(is_code(p, sau) for p in diff if p):
            return 0
    print(
        "check-record: this commit changes code, and `just check` has not passed on the tree it "
        f"commits ({len(code)} code file(s), such as {code[0]}).",
        file=sys.stderr,
    )
    if trees:
        diff = [p for p in git("diff", "--name-only", trees[-1], tree, cwd=cwd).splitlines() if p]
        print("  It differs from the last tree that passed in: " + ", ".join(diff[:8]) + (" …" if len(diff) > 8 else ""), file=sys.stderr)
        print(
            "  The check sees every file that is not ignored, staged or not: stage all its code, "
            "or move what is not part of the commit, then run `just check` again.",
            file=sys.stderr,
        )
    else:
        print("  Run `just check` and commit only when it exits 0 on its own.", file=sys.stderr)
    return 1


# --- the test, run by each repository's `just check` ----------------------------------------


def scenario(sau, me, expect):
    """One throwaway repository, Sau's layout or Null's and Cosmósis's, through every case."""
    with tempfile.TemporaryDirectory() as d:
        repo = pathlib.Path(d)

        def run(*a):
            return git(*a, cwd=repo)

        run("init", "-q", "-b", "main")
        for k, v in (
            ("user.name", "test"),
            ("user.email", "test@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.hooksPath", ".git/hooks"),
        ):
            run("config", k, v)
        (repo / ".gitignore").write_text("/work/\n")
        if sau:
            (repo / "sau" / "tools").mkdir(parents=True)
            (repo / "sau" / "tools" / "dev.py").write_text("# marks a Sau tree\n")
        hook = repo / ".git" / "hooks" / "pre-commit"
        hook.parent.mkdir(parents=True, exist_ok=True)
        hook.write_text(f"#!/bin/sh\nexec {sys.executable} {me} require\n")
        hook.chmod(0o755)
        src = repo / "src.rs"
        src.write_text("fn a() {}\n")
        doc = repo / "docs" / "notes.md"
        doc.parent.mkdir()
        doc.write_text("Notes.\n")

        def commit(msg):
            run("add", "-A")
            r = subprocess.run(["git", "commit", "-q", "-m", msg], cwd=repo, capture_output=True, text=True)
            return r.returncode == 0

        def check(ok=True, edit_during=None):
            start("t", cwd=repo)
            if edit_during:
                edit_during()
            if ok:
                passed("t", cwd=repo, quiet=True)
            else:
                (repo / "work" / "check" / "started-t").unlink()  # a failed check never passes

        tag = "Sau" if sau else "Null and Cosmósis"
        expect(f"{tag}: code with no check", commit("code"), False)
        check(ok=False)
        expect(f"{tag}: code after a failed check", commit("code"), False)
        check()
        expect(f"{tag}: code after a passing check on its tree", commit("code"), True)
        src.write_text("fn a() { b() }\n")
        check()
        src.write_text("fn a() { c() }\n")
        expect(f"{tag}: code edited after its check passed", commit("code"), False)
        (repo / "scratch.rs").write_text("fn x() {}\n")
        check()
        run("add", "src.rs")
        r = subprocess.run(["git", "commit", "-q", "-m", "part"], cwd=repo, capture_output=True, text=True)
        expect(f"{tag}: only part of the tree the check saw", r.returncode == 0, False)
        (repo / "scratch.rs").unlink()
        check(edit_during=lambda: src.write_text("fn a() { d() }\n"))
        expect(f"{tag}: code edited while its check ran", commit("code"), False)
        check()
        expect(f"{tag}: the same code after a check that saw no edit", commit("code"), True)
        doc.write_text("More notes.\n")
        expect(f"{tag}: documents alone, no check", commit("docs"), True)
        src.write_text("fn a() { e() }\n")
        check()
        doc.write_text("Notes written after the check.\n")
        expect(f"{tag}: checked code with documents changed after the check", commit("both"), True)
        src.write_text("fn a() { f() }\n")
        doc.write_text("A report kept for its own commit.\n")
        check()
        run("add", "src.rs")
        r = subprocess.run(["git", "commit", "-q", "-m", "code"], cwd=repo, capture_output=True, text=True)
        expect(f"{tag}: checked code, with a document left unstaged", r.returncode == 0, True)
        (repo / "logo.png").write_bytes(b"\x89PNG")
        expect(f"{tag}: an image alone, no check", commit("image"), not sau)
        # A merge with a conflict, resolved by hand: exempt, as lefthook skips merges.
        run("checkout", "-q", "-b", "side")
        src.write_text("fn side() {}\n")
        check()
        commit("side")
        run("checkout", "-q", "main")
        src.write_text("fn main_side() {}\n")
        check()
        commit("main")
        subprocess.run(["git", "merge", "-q", "side"], cwd=repo, capture_output=True)
        src.write_text("fn merged() {}\n")
        expect(f"{tag}: a conflict resolved by hand, with no check", commit("merge"), False)
        check()
        expect(f"{tag}: the same resolution after a check", commit("merge"), True)
        # A MERGE_HEAD written by hand, naming HEAD itself, exempts nothing.
        src.write_text("fn not_a_merge() {}\n")
        head = run("rev-parse", "HEAD").strip()
        pathlib.Path(run("rev-parse", "--path-format=absolute", "--git-dir").strip(), "MERGE_HEAD").write_text(head + "\n")
        expect(f"{tag}: code with a hand-made MERGE_HEAD equal to HEAD", commit("fake merge"), False)
        pathlib.Path(run("rev-parse", "--path-format=absolute", "--git-dir").strip(), "MERGE_HEAD").write_text(run("rev-parse", "HEAD~1"))
        expect(f"{tag}: code with a hand-made MERGE_HEAD naming HEAD's parent", commit("fake merge"), False)
        pathlib.Path(run("rev-parse", "--path-format=absolute", "--git-dir").strip(), "MERGE_HEAD").unlink()


def fixer_order(me, expect):
    """Through lefthook itself: a pre-commit fixer (Taplo here) re-stages its file only after every
    pre-commit job, so `checked` runs in commit-msg, on the index as it will be committed. A file the
    fixer changes after the check is refused."""
    with tempfile.TemporaryDirectory() as d:
        repo = pathlib.Path(d)

        def run(*a):
            return git(*a, cwd=repo)

        run("init", "-q", "-b", "main")
        for k, v in (("user.name", "test"), ("user.email", "test@example.invalid"), ("commit.gpgsign", "false")):
            run("config", k, v)
        (repo / ".gitignore").write_text("/work/\n")
        (repo / "lefthook.yml").write_text(
            'pre-commit:\n  parallel: true\n  jobs:\n    - name: toml\n      glob: "*.toml"\n'
            "      run: taplo fmt {staged_files}\n      stage_fixed: true\n"
            f"commit-msg:\n  jobs:\n    - name: checked\n      run: {sys.executable} {me} require\n"
        )
        subprocess.run(["lefthook", "install"], cwd=repo, capture_output=True, check=True)
        (repo / "Cargo.toml").write_text('[package]\nname    =   "x"\n')
        run("add", "-A")
        start("t", cwd=repo)
        passed("t", cwd=repo, quiet=True)  # a check that saw the file as written, unformatted
        r = subprocess.run(["git", "commit", "-q", "-m", "toml"], cwd=repo, capture_output=True, text=True)
        expect("lefthook: a file a pre-commit fixer re-stages after the check", r.returncode == 0, False)
        start("t", cwd=repo)
        passed("t", cwd=repo, quiet=True)  # the check run again, on the formatted file
        r = subprocess.run(["git", "commit", "-q", "-m", "toml"], cwd=repo, capture_output=True, text=True)
        expect("lefthook: the same file after a check on its formatted form", r.returncode == 0, True)

        def commit(msg):
            run("add", "-A")
            return subprocess.run(["git", "commit", "-q", "-m", msg], cwd=repo, capture_output=True, text=True).returncode == 0

        def checked():
            start("t", cwd=repo)
            passed("t", cwd=repo, quiet=True)

        (repo / "src.rs").write_text("fn a() {}\n")
        checked()
        commit("a second commit, so HEAD~1 exists")
        git_dir = pathlib.Path(run("rev-parse", "--path-format=absolute", "--git-dir").strip())
        # A MERGE_HEAD written by hand, naming HEAD or an older commit of this branch, merges
        # nothing: lefthook does not skip the commit, and `checked` refuses it.
        for name, rev in (("HEAD", "HEAD"), ("HEAD~1", "HEAD~1")):
            (repo / "src.rs").write_text(f"fn by_hand_{name.replace('~', '_')}() {{}}\n")
            (git_dir / "MERGE_HEAD").write_text(run("rev-parse", rev))
            expect(f"lefthook: unchecked code with a hand-made MERGE_HEAD of {name}", commit("fake merge"), False)
            (git_dir / "MERGE_HEAD").unlink()
            run("reset", "-q", "--hard")
        # A real merge, of a branch whose commits were checked, needs no record of its own.
        run("checkout", "-q", "-b", "side")
        (repo / "side.rs").write_text("fn side() {}\n")
        checked()
        commit("side")
        run("checkout", "-q", "main")
        (repo / "main.rs").write_text("fn main_side() {}\n")
        checked()
        commit("main")
        r = subprocess.run(["git", "merge", "-q", "--no-ff", "-m", "Merge side", "side"], cwd=repo, capture_output=True, text=True)
        expect("lefthook: a real merge, with no check of its own", r.returncode == 0, True)
        # A MERGE_HEAD naming another branch's tip, with other code staged than that merge's.
        run("checkout", "-q", "-b", "other")
        (repo / "other.rs").write_text("fn other() {}\n")
        checked()
        commit("other")
        run("checkout", "-q", "main")
        (git_dir / "MERGE_HEAD").write_text(run("rev-parse", "other"))
        (repo / "sneaked.rs").write_text("fn unchecked() {}\n")
        expect("lefthook: unchecked code with a MERGE_HEAD of another branch's tip", commit("fake merge"), False)
        (git_dir / "MERGE_HEAD").unlink()
        run("reset", "-q", "--hard")
        (repo / "sneaked.rs").unlink(missing_ok=True)
        # A conflict resolved by hand, as `just sync` can leave one: refused until checked.
        run("checkout", "-q", "-b", "theirs")
        (repo / "src.rs").write_text("fn theirs() {}\n")
        checked()
        commit("theirs")
        run("checkout", "-q", "main")
        (repo / "src.rs").write_text("fn ours() {}\n")
        checked()
        commit("ours")
        subprocess.run(["git", "merge", "-q", "theirs"], cwd=repo, capture_output=True)
        (repo / "src.rs").write_text("fn resolved_by_hand() {}\n")
        expect("lefthook: a conflict resolved by hand, with no check", commit("resolve"), False)
        checked()
        expect("lefthook: the same resolution after a check", commit("resolve"), True)


def selftest():
    """Commits in throwaway repositories: refused without a check, after a failed check and after
    one more edit; allowed after a passing check and for documents alone; merges exempt. Then
    this repository's hooks and check must call this file."""
    me = pathlib.Path(__file__).resolve()
    failures = []

    def expect(what, ok, want):
        if ok != want:
            failures.append(f"{what}: {'allowed' if ok else 'refused'}, expected {'allowed' if want else 'refused'}")

    for sau in (False, True):
        scenario(sau, me, expect)
    fixer_order(me, expect)

    root = top()
    if is_sau(root):
        hooks, checks = root / "lefthook.yml", [root / "sau" / "tools" / "check.sh"]
        call = "sau/tools/check-record.py"
    else:
        hooks, checks = root / "lefthook.yml", [root / "justfile"]
        call = "tools/check-record.py"
    text = hooks.read_text()
    if f"{call} require" not in text:
        failures.append(f"{hooks.name} does not run `{call} require` before a commit")
    elif text.index(f"{call} require") < text.index("commit-msg:"):
        failures.append(f"{hooks.name} must run `checked` in commit-msg, after pre-commit's fixers re-stage")
    elif "- merge" in text[text.index("commit-msg:") :]:
        failures.append(f"{hooks.name}'s commit-msg must not skip merges: `checked` decides which merges are real")
    for c in checks:
        text = c.read_text()
        if f"{call} start" not in text or f"{call} pass" not in text:
            failures.append(f"{c.name} does not call `{call} start` and `pass` around the check")
    for f in failures:
        print(f"check-record: {f}", file=sys.stderr)
    if not failures:
        print(
            "check-record: selftest passed (refused without a check, after a failure, after an edit; "
            "allowed after a pass and for documents; merges exempt; hooks wired)"
        )
    return 1 if failures else 0


def main():
    args = sys.argv[1:]
    if args[:1] == ["start"] and len(args) == 2:
        return start(args[1])
    if args[:1] == ["pass"] and len(args) == 2:
        return passed(args[1])
    if args == ["require"]:
        return require()
    if args == ["selftest"]:
        return selftest()
    sys.exit(__doc__)


if __name__ == "__main__":
    sys.exit(main())
