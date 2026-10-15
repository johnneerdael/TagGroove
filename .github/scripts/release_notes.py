#!/usr/bin/env python3
"""Validate and collect public PR notes; adapted from ProjectM-TV release_notes.py."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import json
import os
from pathlib import Path
import re
import subprocess
import sys
from typing import Callable


@dataclass(frozen=True)
class Commit:
    sha: str
    subject: str
    body: str


PullRequest = dict[str, object]
PullLookup = Callable[[str], list[PullRequest]]


def strip_comments(text: str) -> str:
    return re.sub(r"<!--.*?(?:-->|\Z)", "", text, flags=re.DOTALL)


def markdown_lines(text: str) -> list[tuple[str, int, str]]:
    """Return lines with ATX heading metadata, ignoring headings in fenced code."""
    result = []
    fence = ""
    for line in strip_comments(text).splitlines():
        marker = re.match(r"^\s{0,3}(`{3,}|~{3,})", line)
        if marker:
            token = marker.group(1)
            if not fence:
                fence = token
            elif token[0] == fence[0] and len(token) >= len(fence):
                fence = ""
            result.append((line, 0, ""))
            continue
        heading = re.match(r"^\s{0,3}(#{1,6})\s+(.+?)\s*#*\s*$", line) if not fence else None
        result.append((line, len(heading.group(1)) if heading else 0,
                       heading.group(2).strip("*_ ") if heading else ""))
    return result


def release_section(body: str) -> str | None:
    lines = markdown_lines(body)
    start = next((index for index, (_, depth, title) in enumerate(lines)
                  if depth == 2 and re.fullmatch(r"release[ -]+notes\s*:?", title, re.IGNORECASE)), None)
    if start is None:
        return None
    level = lines[start][1]
    end = next((index for index in range(start + 1, len(lines))
                if lines[index][1] and lines[index][1] <= level), len(lines))
    return "\n".join(line for line, _, _ in lines[start + 1:end]).strip()


def plain_content(line: str) -> str:
    line = re.sub(r"^\s*(?:[-*+] |\d+[.)] )", "", line)
    line = re.sub(r"^\[[ xX]\]\s*", "", line)
    line = line.replace("**", "").replace("__", "")
    return line.strip(" \t*_`[]:.!-").casefold()


def is_vague(content: str) -> bool:
    return bool(re.fullmatch(
        r"(?:no user[ -]visible changes?|no changes?|none|n/?a|not applicable|"
        r"maintenance|internal changes?|updates?|bug fixes?|miscellaneous(?: changes)?|"
        r"todo|tbd|coming soon)", content))


def has_template_placeholder(line: str) -> bool:
    # Only recognize enclosed template instructions, never Markdown link labels.
    for match in re.finditer(r"\[([^\]\n]+)\](?!\s*[\[(])|<([^>\n]+)>", line):
        text = (match[1] or match[2]).strip()
        if re.fullmatch(
                r"(?:describe|explain|insert|add|replace|write)"
                r"(?:\s+(?:this(?:\s+placeholder)?\s+)?with)?\s+"
                r"(?:(?:the|actual|your)\s+)?(?:changes?|release notes)(?:\s+here)?[.!]?",
                text, re.IGNORECASE):
            return True
    return False


def validate_release_notes(body: str) -> str:
    section = release_section(body)
    if section is None:
        raise ValueError("PR body must include a ## Release notes section.")
    content = []
    internal = []
    in_internal = False
    internal_level = 0
    for line, depth, title in markdown_lines(section):
        if re.search(r"\b(?:TODO|TBD)\b", line, re.IGNORECASE):
            raise ValueError("Release notes must replace TODO/TBD placeholders with actual changes.")
        if has_template_placeholder(line):
            raise ValueError("Release notes must replace template placeholders with actual changes.")
        if depth:
            if internal_level and depth <= internal_level:
                in_internal = False
                internal_level = 0
            if re.match(r"internal\b", title, re.IGNORECASE):
                in_internal = True
                internal_level = depth
            continue
        value = plain_content(line)
        if not value or re.fullmatch(r"[-*+\s]+", value):
            continue
        label = re.match(r"internal(?: changes?)?\s*:\s*(.*)", value)
        if label:
            in_internal = True
            value = label.group(1).strip()
        if re.fullmatch(r"n/?a|not applicable", value):
            raise ValueError("Release notes must replace N/A placeholders with actual changes.")
        if value:
            content.append(value)
            if in_internal:
                internal.append(value)
    if not content or all(is_vague(value) for value in content):
        raise ValueError("Release notes must describe a concrete change, including internal changes when applicable.")
    if any(re.fullmatch(r"no user[ -]visible changes?", value) for value in content):
        if not any(not is_vague(value) for value in internal):
            raise ValueError("No user-visible changes requires an Internal explanation of the actual change.")
    return section


def historical_description(body: str) -> str:
    """Keep historical change descriptions, excluding review-only sections."""
    kept = []
    skipped_level = 0
    skipped_label = False
    review_section = re.compile(r"\b(?:validation|tests?|testing|checklist|test\s*plan|screenshots?|review|verification|not changed)\b", re.IGNORECASE)
    for line, depth, title in markdown_lines(body):
        if re.search(r"(?:generated (?:with|by)|co-authored-by:|\b(?:claude|codex) (?:code|agent)\b)", line, re.IGNORECASE):
            break
        if re.search(r"!\[|<img\b|img\.shields\.io/", line, re.IGNORECASE):
            continue
        if depth:
            if skipped_level and depth > skipped_level:
                continue
            skipped_level = 0
            skipped_label = False
            if review_section.search(title):
                skipped_level = depth
                continue
            if title.casefold() in {"summary", "description", "changes", "what changed"}:
                continue
        if skipped_level:
            continue
        label_text = line.strip().replace("**", "").replace("__", "")
        label = re.fullmatch(r"([A-Za-z][A-Za-z /-]{0,60}):\s*(.*)", label_text)
        if label:
            skipped_label = bool(review_section.search(label.group(1)))
        if skipped_label:
            continue
        if re.match(r"\s*[-*+]\s+\[[ xX]\]", line) or re.match(r"\s*(?:---+|___+|\*\*\*+)\s*$", line):
            continue
        kept.append(line)
    result = "\n".join(kept).strip()
    return result if any(not is_vague(plain_content(line)) for line in kept if plain_content(line)) else ""


def nested_markdown(text: str) -> str:
    lines = markdown_lines(text)
    levels = [depth for _, depth, _ in lines if depth]
    offset = 4 - min(levels) if levels else 0
    return "\n".join(re.sub(r"^(\s{0,3})#{1,6}", lambda match: match.group(1) + "#" * min(6, depth + offset), line)
                     if depth else line for line, depth, _ in lines)


def render_release_notes(repo: str, commits: list[Commit], pull_requests: PullLookup,
                         required_shas: set[str]) -> str:
    parts, seen = ["## Changes"], set()
    for commit in commits:
        merged = [pr for pr in pull_requests(commit.sha) if pr.get("merged_at")]
        if merged:
            for pr in merged:
                number = int(pr["number"])
                if number in seen:
                    continue
                seen.add(number)
                body = str(pr.get("body") or "")
                section = release_section(body)
                if commit.sha in required_shas and section is None:
                    raise ValueError(f"Merged PR #{number} must include a ## Release notes section.")
                text = validate_release_notes(body) if commit.sha in required_shas else historical_description(body)
                title = " ".join(str(pr["title"]).splitlines())
                parts.append(f"### {title} ([#{number}](https://github.com/{repo}/pull/{number}))")
                if text:
                    parts.append(nested_markdown(text))
        else:
            parts.append(f"### {commit.subject} ([{commit.sha[:7]}](https://github.com/{repo}/commit/{commit.sha}))")
            body = historical_description(commit.body)
            if body:
                parts.append(nested_markdown(body))
    if not commits:
        parts.append("No commits in this release range.")
    return "\n\n".join(parts) + "\n"


def git(checkout: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(checkout), *args], check=True,
                          capture_output=True, text=True).stdout.strip()


def collect_commits(checkout: Path, sha: str, previous_tag: str | None) -> list[Commit]:
    current = git(checkout, "rev-parse", "--verify", "--end-of-options", f"{sha}^{{commit}}")
    revision = current
    if previous_tag:
        if not re.fullmatch(r"v[0-9][0-9A-Za-z.+-]*", previous_tag):
            raise ValueError("Previous release must be a version tag")
        previous = git(checkout, "rev-parse", "--verify", "--end-of-options", f"refs/tags/{previous_tag}^{{commit}}")
        if previous not in git(checkout, "rev-list", "--first-parent", current, "--").splitlines():
            raise ValueError("Previous release tag is not on first-parent history")
        revision = f"{previous}..{current}"
    commits = []
    for commit_sha in git(checkout, "rev-list", "--first-parent", "--reverse", revision, "--").splitlines():
        subject, _, body = git(checkout, "show", "-s", "--format=%s%n%b", commit_sha, "--").partition("\n")
        commits.append(Commit(commit_sha, subject, body.strip()))
    return commits


def required_note_commits(checkout: Path, sha: str, baseline: str) -> set[str]:
    if not re.fullmatch(r"[0-9a-f]{40}", baseline):
        raise ValueError("Release note baseline must be a full commit SHA")
    history = git(checkout, "rev-list", "--first-parent", "--reverse", sha, "--").splitlines()
    if baseline not in history:
        # Explicit prereleases from old history have no newly governed merges.
        if sha in git(checkout, "rev-list", "--first-parent", baseline, "--").splitlines():
            return set()
        raise ValueError("Release note baseline is not on first-parent history")
    return set(history[history.index(baseline) + 1:])


def associated_pull_requests(repo: str, sha: str) -> list[PullRequest]:
    output = subprocess.run(["gh", "api", "--method", "GET", "--paginate", "--slurp",
                             f"repos/{repo}/commits/{sha}/pulls?per_page=100"],
                            check=True, text=True, capture_output=True).stdout
    return [item for page in json.loads(output) for item in page]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    validate = commands.add_parser("validate")
    validate.add_argument("--event-file", type=Path, required=True)
    generate = commands.add_parser("generate")
    generate.add_argument("--checkout", type=Path, default=Path.cwd())
    generate.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY", ""))
    generate.add_argument("--sha", required=True)
    generate.add_argument("--previous-tag")
    args = parser.parse_args()
    try:
        if args.command == "validate":
            event = json.loads(args.event_file.read_text())
            validate_release_notes(event["pull_request"].get("body") or "")
            print("Release notes describe a concrete change.")
        else:
            if not re.fullmatch(r"[0-9a-f]{40}", args.sha) or not re.fullmatch(r"[\w.-]+/[\w.-]+", args.repo):
                raise ValueError("A full source SHA and owner/repository are required")
            baseline = (args.checkout / ".github/release-notes-baseline").read_text().strip()
            commits = collect_commits(args.checkout, args.sha, args.previous_tag)
            required = required_note_commits(args.checkout, args.sha, baseline)
            print(render_release_notes(args.repo, commits, lambda sha: associated_pull_requests(args.repo, sha), required), end="")
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"Release notes error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
