#!/usr/bin/env python3
"""Compile each task set in task_sets.csv into a standalone PDF.

For every row, pulls the *full* issue body (task_sets.csv only carries a
220-char excerpt) via `gh issue view --json body`, assembles one Markdown
document per set (Task 1..10, in task_sets.csv's row order), and renders it
to PDF with pandoc + pdflatex.

These PDFs currently include the issue number, URL, and category/size tags
-- i.e. they're moderator/reference copies, not participant hand-outs. If
these go in front of a participant during the actual session, the issue
number and URL should probably be stripped first: OEWN's tracker is public,
so a participant could look up the real merged fix and defeat the point of
timing them. Ask for a sanitized --participant-safe variant if you want one.

Usage:
    python3 build_task_pdfs.py [--in task_sets.csv] [--repo OWNER/NAME]
                                [--out-dir .]

Requires `gh` (authenticated) and `pandoc` with a working LaTeX engine.
"""
import argparse
import csv
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path


def fetch_body(repo: str, number: str) -> str:
    proc = subprocess.run(
        ["gh", "issue", "view", number, "--repo", repo, "--json", "body",
         "-q", ".body"],
        capture_output=True, text=True,
    )
    if proc.returncode != 0:
        print(f"WARNING: failed to fetch issue #{number}: {proc.stderr.strip()}",
              file=sys.stderr)
        return "*(failed to fetch issue body -- see stderr)*"
    return proc.stdout.strip() or "*(empty issue body)*"


def build_markdown(set_num: int, rows: list[dict], repo: str) -> str:
    lines = [
        f"---",
        f'title: "EWE User Study \\u2014 Task Set {set_num}"',
        f"---",
        "",
        f"# Task Set {set_num}",
        "",
        "Moderator/reference copy -- includes issue number, URL, and the "
        "gold-fix PR link. See `task_sets.csv` for the same metadata in "
        "tabular form.",
        "",
    ]
    for i, row in enumerate(rows, start=1):
        print(f"  fetching #{row['number']} ({row['title']})...")
        body = fetch_body(repo, row["number"])
        lines += [
            f"## Task {i} — {row['title']}",
            "",
            f"**Issue:** [#{row['number']}]({row['url']})  \\",
            f"**Category:** {row['guessed_category']}  \\",
            f"**Size:** {row['size_bucket']}  \\",
            f"**Gold fix (moderator only):** [{row['pr_url']}]({row['pr_url']})",
            "",
            body,
            "",
            "\\newpage",
            "",
        ]
    return "\n".join(lines)


UNICODE_ERROR_RE = re.compile(r"Unicode character (.) \(U\+([0-9A-Fa-f]+)\)")


def render_pdf(md_path: Path, pdf_path: Path, max_attempts: int = 20) -> None:
    """Run pandoc, and if pdflatex chokes on a specific Unicode character,
    patch just that character out of the .md file and retry.

    pdflatex/inputenc already renders ordinary typographic Unicode fine
    (curly quotes, em dashes, accented letters, ...) via its built-in
    mapping table, so we must NOT blanket-replace anything outside Latin-1
    -- that was the previous bug (turned every curly quote into a visible
    [U+201C] marker). Only genuinely unsupported characters (e.g. U+27EA, a
    math angle bracket with no LaTeX-safe fallback) need patching, and we
    only find out which ones by actually trying to compile.
    """
    patched = set()
    for attempt in range(max_attempts):
        proc = subprocess.run(
            ["pandoc", str(md_path), "-o", str(pdf_path),
             "--pdf-engine=pdflatex",
             "-V", "geometry:margin=1in",
             "-V", "colorlinks=true",
             "--toc"],
            capture_output=True, text=True,
        )
        if proc.returncode == 0:
            if patched:
                print(f"  (patched {len(patched)} unsupported char(s): "
                      f"{''.join(sorted(patched))!r})")
            return

        m = UNICODE_ERROR_RE.search(proc.stderr)
        if not m:
            print(f"pandoc failed for {md_path.name}:\n{proc.stderr}", file=sys.stderr)
            raise SystemExit(1)
        bad_char, codepoint = m.group(1), m.group(2).upper()
        if bad_char in patched:
            print(f"pandoc still failing on U+{codepoint} after patching it "
                  f"once -- giving up:\n{proc.stderr}", file=sys.stderr)
            raise SystemExit(1)
        patched.add(bad_char)

        text = md_path.read_text(encoding="utf-8")
        md_path.write_text(text.replace(bad_char, f"[U+{codepoint}]"), encoding="utf-8")

    raise SystemExit(f"gave up on {md_path.name} after {max_attempts} unicode patch attempts")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--in", dest="infile", default="task_sets.csv")
    ap.add_argument("--repo", default="globalwordnet/english-wordnet")
    ap.add_argument("--out-dir", default=".")
    args = ap.parse_args()

    with open(args.infile, newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    by_set = defaultdict(list)
    for r in rows:
        by_set[int(r["set"])].append(r)

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    for set_num in sorted(by_set):
        print(f"Set {set_num}: {len(by_set[set_num])} tasks")
        md = build_markdown(set_num, by_set[set_num], args.repo)
        md_path = out_dir / f"task_set_{set_num}.md"
        md_path.write_text(md, encoding="utf-8")

        pdf_path = out_dir / f"task_set_{set_num}.pdf"
        render_pdf(md_path, pdf_path)
        print(f"  -> {pdf_path}")


if __name__ == "__main__":
    main()
