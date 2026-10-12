#!/usr/bin/env python3
"""Mine candidate user-study tasks from globalwordnet/english-wordnet issues.

Pulls closed issues (newest first) via the GitHub GraphQL API, keeps only
those actually resolved by a *merged* pull request (so each candidate task
has a real gold fix a moderator can score against), tags each with a rough
keyword-based category and a size bucket from the fixing PR's diff, and
writes the result to CSV for manual review.

Two content filters are on by default, based on manual review of an earlier
pull (e.g. #1317 "release format", a systemic multi-file relabelling issue;
#1110/#1358, feature requests / tool bugs whose fix never touches wordnet
data at all):

  - Issues carrying a label in --exclude-labels (default: "release format")
    are dropped -- these tend to be systemic/format-policy issues, not a
    single self-contained content edit.
  - Issues whose fixing PR touches no file under --data-path-prefix
    (default: "src/yaml/") are dropped by default (--include-non-content to
    keep them) -- these are tooling/docs/API fixes, not wordnet content.

A PR that touches both data and non-data files (e.g. a content fix plus a
validation-script change) is kept but flagged via pr_non_data_files_changed
so you can judge it yourself.

This does NOT pick the final 10 tasks for you -- categorization and size are
heuristics meant to help you stratify/skim, not ground truth. Task
suitability for an in-office timed study (clear scope, no specialist
knowledge needed, doable in a few minutes) is a judgment call to make by
reading the issue text yourself.

Usage:
    python3 mine_issues.py [--repo OWNER/NAME] [--max-issues N]
                            [--since YYYY-MM-DD] [--include-open]
                            [--exclude-labels "release format,..."]
                            [--data-path-prefix src/yaml/]
                            [--include-non-content]
                            [--out FILE.csv]

Requires the `gh` CLI, authenticated (`gh auth status`).
"""
import argparse
import csv
import json
import re
import subprocess
import sys
from datetime import datetime, timezone

GRAPHQL_QUERY = """
query($owner: String!, $name: String!, $states: [IssueState!]!, $cursor: String) {
  repository(owner: $owner, name: $name) {
    issues(states: $states, first: 50, orderBy: {field: CREATED_AT, direction: DESC}, after: $cursor) {
      pageInfo { hasNextPage endCursor }
      nodes {
        number
        title
        body
        url
        createdAt
        closedAt
        labels(first: 10) { nodes { name } }
        timelineItems(last: 20, itemTypes: [CLOSED_EVENT]) {
          nodes {
            ... on ClosedEvent {
              closer {
                ... on PullRequest {
                  number
                  url
                  merged
                  mergedAt
                  additions
                  deletions
                  changedFiles
                  files(first: 100) {
                    totalCount
                    nodes { path }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}
"""

# Rough keyword -> category map, checked against "title \n body" lowercased.
# Order matters: first match wins.
CATEGORY_KEYWORDS = [
    ("duplicate_or_merge", ["duplicate", "merge", "same sense", "same synset"]),
    ("deprecation", ["deprecat"]),
    ("relation", ["hypernym", "hyponym", "antonym", "meronym", "holonym",
                  "relation", "similar to", "derivation"]),
    ("pos_error", ["wrong pos", "part of speech", "should be a verb",
                   "should be a noun", "should be an adjective", "wrong part of speech"]),
    ("definition", ["definition", "gloss", "wording"]),
    ("new_lemma_or_sense", ["missing word", "missing sense", "new sense",
                            "add lemma", "new entry", "not in wordnet",
                            "doesn't exist", "does not exist"]),
    ("example", ["example sentence", "example is wrong", "bad example"]),
]

# Title/body keywords suggesting the issue is *about* the tooling/generation
# pipeline rather than a specific piece of wordnet content -- even when its
# eventual fix happens to touch a data file (e.g. #1358: a generator bug that
# mislabelled adjectives, fixed by patching both the script and the entries
# it had already corrupted). Not auto-excluded by default: this is a fuzzy
# signal meant to surface candidates for a human "is this really a content
# task?" check, not a hard filter.
#
# Matched with a leading word-boundary only (not a full-word match), so e.g.
# "script" also catches "scripts"/"scripting" but NOT "description" (the 'e'
# before "script" there isn't a word boundary).
TOOLING_KEYWORDS = [
    "tool", "script", "validator", "pipeline", "generat", "autogenerat",
    "json api", "rest api", "export format", "release process",
    "build process", "regex", "parser",
]
_TOOLING_PATTERN = re.compile(
    r"\b(?:" + "|".join(re.escape(kw) for kw in TOOLING_KEYWORDS) + r")"
)


def run_gh_graphql(query: str, owner: str, name: str, states: list[str],
                    cursor: str | None) -> dict:
    args = ["gh", "api", "graphql", "-f", f"query={query}",
            "-f", f"owner={owner}", "-f", f"name={name}"]
    for state in states:
        args += ["-f", f"states[]={state}"]
    if cursor:
        args += ["-f", f"cursor={cursor}"]
    proc = subprocess.run(args, capture_output=True, text=True)
    if proc.returncode != 0:
        print(proc.stderr, file=sys.stderr)
        raise SystemExit(f"gh api graphql failed (exit {proc.returncode})")
    return json.loads(proc.stdout)


def guess_category(title: str, body: str) -> str:
    text = f"{title}\n{body or ''}".lower()
    for category, keywords in CATEGORY_KEYWORDS:
        if any(kw in text for kw in keywords):
            return category
    return "other"


def looks_like_tooling_issue(title: str, body: str) -> bool:
    text = f"{title}\n{body or ''}".lower()
    return _TOOLING_PATTERN.search(text) is not None


def size_bucket(pr: dict) -> str:
    if not pr:
        return "unknown"
    changed = (pr.get("additions") or 0) + (pr.get("deletions") or 0)
    if changed <= 4:
        return "tiny"
    if changed <= 20:
        return "small"
    if changed <= 80:
        return "medium"
    return "large"


def normalize_label(label: str) -> str:
    return re.sub(r"[-_]+", " ", label.strip().lower())


def classify_files(pr: dict, data_path_prefix: str) -> dict:
    """Split a PR's changed file paths into data (wordnet content) vs. not.

    Only the first 100 file paths are available from the API; if the PR
    changed more than that (pr["files"]["totalCount"] > len(nodes)), the
    non-data count is a lower bound -- flagged via pr_files_truncated.
    """
    if not pr:
        return {"data_files_changed": 0, "non_data_files_changed": 0,
                "pr_files_truncated": False}
    paths = [n["path"] for n in pr.get("files", {}).get("nodes", [])]
    total = pr.get("files", {}).get("totalCount", len(paths))
    data_count = sum(1 for p in paths if p.startswith(data_path_prefix))
    non_data_count = len(paths) - data_count
    return {
        "data_files_changed": data_count,
        "non_data_files_changed": non_data_count,
        "pr_files_truncated": total > len(paths),
    }


def find_merged_closer(issue: dict) -> dict | None:
    for node in issue.get("timelineItems", {}).get("nodes", []):
        closer = (node or {}).get("closer")
        if closer and closer.get("merged"):
            return closer
    return None


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--repo", default="globalwordnet/english-wordnet")
    ap.add_argument("--max-issues", type=int, default=400,
                     help="stop after scanning this many issues (pages of 50)")
    ap.add_argument("--since", default=None,
                     help="only keep issues created on/after this date (YYYY-MM-DD)")
    ap.add_argument("--include-open", action="store_true",
                     help="also fetch open issues (no gold fix available yet -- "
                          "useful for Study A's contamination-proof slice, not "
                          "for Study B's timed tasks since there's no way to "
                          "score correctness against a known-good fix)")
    ap.add_argument("--exclude-labels", default="release format",
                     help="comma-separated labels to drop (case/hyphen-insensitive); "
                          "empty string to disable")
    ap.add_argument("--data-path-prefix", default="src/yaml/",
                     help="path prefix (within the repo) identifying wordnet "
                          "content files, used to filter out tooling/docs-only fixes")
    ap.add_argument("--include-non-content", action="store_true",
                     help="keep closed issues whose fixing PR touches no file "
                          "under --data-path-prefix (tooling/docs/API fixes, "
                          "e.g. a JSON API feature request)")
    ap.add_argument("--exclude-tooling-keywords", action="store_true",
                     help="also drop issues whose title/body reads as being "
                          "about the generation tooling itself (script/"
                          "validator/pipeline bugs), even if the fix happened "
                          "to touch a data file -- e.g. #1358. Off by default "
                          "since this keyword match is fuzzy; the column is "
                          "always included in the CSV for manual review either way.")
    ap.add_argument("--out", default="candidate_issues.csv")
    args = ap.parse_args()

    owner, name = args.repo.split("/")
    since = datetime.fromisoformat(args.since).replace(tzinfo=timezone.utc) if args.since else None
    exclude_labels = {normalize_label(l) for l in args.exclude_labels.split(",") if l.strip()}

    states = ["CLOSED", "OPEN"] if args.include_open else ["CLOSED"]

    rows = []
    scanned = 0
    cursor = None
    while scanned < args.max_issues:
        data = run_gh_graphql(GRAPHQL_QUERY, owner, name, states, cursor)
        if "errors" in data:
            print(json.dumps(data["errors"], indent=2), file=sys.stderr)
            raise SystemExit("GraphQL query returned errors")

        issues_conn = data["data"]["repository"]["issues"]
        nodes = issues_conn["nodes"]
        if not nodes:
            break

        for issue in nodes:
            scanned += 1
            created_at = datetime.fromisoformat(issue["createdAt"].replace("Z", "+00:00"))
            if since and created_at < since:
                continue

            is_open = issue["closedAt"] is None
            pr = None if is_open else find_merged_closer(issue)
            if not is_open and pr is None:
                continue  # closed without a merged fixing PR -- no gold fix, skip

            labels = [l["name"] for l in issue["labels"]["nodes"]]
            if exclude_labels and any(normalize_label(l) in exclude_labels for l in labels):
                continue

            file_class = classify_files(pr, args.data_path_prefix)
            if (not is_open and not args.include_non_content
                    and file_class["data_files_changed"] == 0):
                continue  # fixing PR never touched wordnet content -- tooling/docs/API

            title = issue["title"] or ""
            body = issue["body"] or ""
            if args.exclude_tooling_keywords and looks_like_tooling_issue(title, body):
                continue
            body_excerpt = re.sub(r"\s+", " ", body).strip()[:220]

            rows.append({
                "number": issue["number"],
                "title": title,
                "url": issue["url"],
                "state": "OPEN" if is_open else "CLOSED",
                "created_at": issue["createdAt"],
                "closed_at": issue["closedAt"] or "",
                "labels": ";".join(labels),
                "guessed_category": guess_category(title, body),
                "possibly_tooling_issue": looks_like_tooling_issue(title, body),
                "pr_number": pr["number"] if pr else "",
                "pr_url": pr["url"] if pr else "",
                "pr_merged_at": pr["mergedAt"] if pr else "",
                "pr_changed_files": pr["changedFiles"] if pr else "",
                "pr_data_files_changed": file_class["data_files_changed"] if pr else "",
                "pr_non_data_files_changed": file_class["non_data_files_changed"] if pr else "",
                "pr_files_truncated": file_class["pr_files_truncated"] if pr else "",
                "pr_additions": pr["additions"] if pr else "",
                "pr_deletions": pr["deletions"] if pr else "",
                "size_bucket": size_bucket(pr) if pr else "n/a (open)",
                "body_excerpt": body_excerpt,
            })

        if not issues_conn["pageInfo"]["hasNextPage"]:
            break
        cursor = issues_conn["pageInfo"]["endCursor"]

    fieldnames = ["number", "title", "url", "state", "created_at", "closed_at",
                  "labels", "guessed_category", "possibly_tooling_issue",
                  "pr_number", "pr_url", "pr_merged_at", "pr_changed_files",
                  "pr_data_files_changed", "pr_non_data_files_changed",
                  "pr_files_truncated", "pr_additions", "pr_deletions",
                  "size_bucket", "body_excerpt"]
    with open(args.out, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print(f"Scanned {scanned} issues, kept {len(rows)} candidates -> {args.out}")
    by_cat = {}
    by_size = {}
    for r in rows:
        by_cat[r["guessed_category"]] = by_cat.get(r["guessed_category"], 0) + 1
        by_size[r["size_bucket"]] = by_size.get(r["size_bucket"], 0) + 1
    print("By guessed category:", json.dumps(by_cat, indent=2))
    print("By size bucket:", json.dumps(by_size, indent=2))
    mixed = sum(1 for r in rows if isinstance(r["pr_non_data_files_changed"], int)
                and r["pr_non_data_files_changed"] > 0)
    if mixed:
        print(f"{mixed} candidate(s) also touch non-data files (e.g. scripts/) "
              f"alongside wordnet content -- check pr_non_data_files_changed "
              f"before picking those.")
    tooling = sum(1 for r in rows if r["possibly_tooling_issue"])
    if tooling:
        print(f"{tooling} candidate(s) flagged possibly_tooling_issue=True "
              f"(reads as being about the generation pipeline, e.g. #1358) "
              f"-- review before picking; re-run with --exclude-tooling-keywords "
              f"to drop them automatically.")


if __name__ == "__main__":
    main()
