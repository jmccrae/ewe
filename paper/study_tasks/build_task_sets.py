#!/usr/bin/env python3
"""Split candidate_issues.csv into 3 matched sets of 10 tasks each for the
Study B user study (one set per condition, rotated via a 3x3 Latin square
across the 3 participants -- see paper/study_materials/survey.tex).

Selection is restricted to issues that look reasonable for an in-office
timed single-task exercise (excludes 'large' diffs and anything flagged
possibly_tooling_issue), then stratified by (guessed_category, size_bucket)
so all 3 sets end up with a comparable mix of edit types and sizes -- the
whole point being that no set is systematically easier than another,
regardless of which condition it's rotated into.

This is a heuristic starting allocation, not a final answer: guessed_category
is keyword-based and can misclassify (as seen with #1358 previously), so
read the titles/urls in the output before locking in the 30. Swap any bad
pick for another same-(category,size_bucket) row from candidate_issues.csv
by hand -- the CSV has plenty of headroom in every stratum used here.

Usage:
    python3 build_task_sets.py [--in candidate_issues.csv] [--out task_sets.csv]
"""
import argparse
import csv
from collections import defaultdict

# Explicit per-stratum take-counts and their round-robin split across the 3
# sets (set index 0/1/2). Chosen by hand against this pool's actual
# category x size_bucket counts (see mine_issues.py output) so that:
#   - definition, duplicate_or_merge, relation, other all appear in every set
#   - the two rarest categories (deprecation: 2 available, new_lemma_or_sense:
#     1 available) go to different sets rather than being dropped, since
#     losing them would remove a whole edit-type from the study entirely
#   - each set ends up at exactly 10 tasks with a reasonably even tiny/
#     small/medium spread
PLAN = [
    # (category, size_bucket, how many to take, [set indices to assign, in order])
    ("definition", "tiny", 3, [0, 1, 2]),
    ("definition", "small", 3, [0, 1, 2]),
    ("definition", "medium", 3, [0, 1, 2]),
    ("duplicate_or_merge", "medium", 2, [0, 1]),
    ("duplicate_or_merge", "small", 1, [2]),
    ("deprecation", "medium", 2, [0, 1]),
    ("new_lemma_or_sense", "small", 1, [2]),
    ("other", "tiny", 2, [0, 1]),
    ("other", "small", 2, [1, 2]),
    ("other", "medium", 2, [2, 0]),
    ("relation", "tiny", 3, [0, 1, 2]),
    ("relation", "small", 3, [0, 1, 2]),
    ("relation", "medium", 3, [0, 1, 2]),
]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--in", dest="infile", default="candidate_issues.csv")
    ap.add_argument("--out", default="task_sets.csv")
    args = ap.parse_args()

    with open(args.infile, newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    strata = defaultdict(list)
    for r in rows:
        if r["size_bucket"] not in ("tiny", "small", "medium"):
            continue
        if r["possibly_tooling_issue"] == "True":
            continue
        strata[(r["guessed_category"], r["size_bucket"])].append(r)

    for key in strata:
        strata[key].sort(key=lambda r: int(r["number"]), reverse=True)

    sets = {0: [], 1: [], 2: []}
    warnings = []
    for category, size_bucket, take, set_order in PLAN:
        available = strata.get((category, size_bucket), [])
        if len(available) < take:
            warnings.append(
                f"WARNING: wanted {take} from ({category}, {size_bucket}) "
                f"but only {len(available)} available -- adjust PLAN"
            )
            take = len(available)
        chosen = available[:take]
        for row, set_idx in zip(chosen, set_order):
            sets[set_idx].append(row)

    for w in warnings:
        print(w)

    fieldnames = ["set", "number", "title", "url", "guessed_category",
                  "size_bucket", "pr_url", "body_excerpt"]
    with open(args.out, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        for set_idx in (0, 1, 2):
            for r in sets[set_idx]:
                writer.writerow({
                    "set": set_idx + 1,
                    "number": r["number"],
                    "title": r["title"],
                    "url": r["url"],
                    "guessed_category": r["guessed_category"],
                    "size_bucket": r["size_bucket"],
                    "pr_url": r["pr_url"],
                    "body_excerpt": r["body_excerpt"],
                })

    print(f"\nWrote {sum(len(v) for v in sets.values())} tasks -> {args.out}")
    for set_idx in (0, 1, 2):
        cat_counts = defaultdict(int)
        size_counts = defaultdict(int)
        for r in sets[set_idx]:
            cat_counts[r["guessed_category"]] += 1
            size_counts[r["size_bucket"]] += 1
        print(f"\nSet {set_idx + 1} ({len(sets[set_idx])} tasks):")
        print("  category:", dict(cat_counts))
        print("  size:    ", dict(size_counts))


if __name__ == "__main__":
    main()
