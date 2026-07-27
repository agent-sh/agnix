import json
import re
import sys
from collections import Counter, defaultdict
from typing import Optional
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RULES_PATH = ROOT / "knowledge-base" / "rules.json"


def parse_int(value: str) -> Optional[int]:
    match = re.search(r"\d+", value)
    if not match:
        return None
    return int(match.group(0))


def find_table(lines: list[str], heading: str) -> tuple[int, int]:
    """Return the [start, end) line range of the table under `heading`."""
    for idx, line in enumerate(lines):
        if heading in line:
            start = idx + 1
            while start < len(lines) and not lines[start].lstrip().startswith("|"):
                start += 1
            end = start
            while end < len(lines) and lines[end].lstrip().startswith("|"):
                end += 1
            if start == end:
                raise ValueError(f"No table found after heading '{heading}'")
            return start, end
    raise ValueError(f"Heading '{heading}' not found")


def extract_table(lines: list[str], heading: str) -> list[str]:
    start, end = find_table(lines, heading)
    return lines[start:end]


def parse_category_table(table_lines: list[str]) -> dict[str, list[int]]:
    """Parse the Rules / HIGH / MEDIUM / LOW / Auto-Fix columns of a
    category table. The Auto-Fix column is included so it is verified
    too: it went unchecked for a long time and drifted (Claude Skills
    read 13 against 11 autofixable rules)."""
    rows: dict[str, list[int]] = {}
    for line in table_lines:
        if not line.lstrip().startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 6:
            continue
        label = re.sub(r"[`*]", "", cells[0]).strip()
        if label.lower() in {"category", "total rules"}:
            continue
        counts = [
            parse_int(cells[1]),
            parse_int(cells[2]),
            parse_int(cells[3]),
            parse_int(cells[4]),
            parse_int(cells[5]),
        ]
        if any(count is None for count in counts):
            continue
        rows[label] = [int(count) for count in counts if count is not None]
    return rows


def parse_spec_table(table_lines: list[str]) -> dict[str, int]:
    rows: dict[str, int] = {}
    for line in table_lines:
        if not line.lstrip().startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 3:
            continue
        label = cells[0].strip()
        if label.lower() == "type":
            continue
        count = parse_int(cells[2])
        if count is None:
            continue
        rows[label] = count
    return rows


def require_counts_in_text(
    path: Path, pattern: str, expected: int, errors: list[str], fix: bool = False
) -> None:
    text = path.read_text(encoding="utf-8")
    matches = re.findall(pattern, text)
    if not matches:
        errors.append(f"{path}: missing pattern '{pattern}'")
        return
    stale = [m for m in matches if int(m) != expected]
    if not stale:
        return
    if not fix:
        for match in stale:
            errors.append(
                f"{path}: expected {expected} for pattern '{pattern}', found {match}"
            )
        return

    # Rewrite only the captured number, leaving the rest of the match
    # (and all surrounding prose) byte-identical.
    def replace(match: re.Match[str]) -> str:
        whole = match.group(0)
        if int(match.group(1)) == expected:
            return whole
        span_start, span_end = match.span(1)
        offset = match.start()
        return (
            whole[: span_start - offset] + str(expected) + whole[span_end - offset :]
        )

    path.write_text(re.sub(pattern, replace, text), encoding="utf-8")
    print(f"[fixed] {path}: {pattern} -> {expected}")


def set_row_counts(line: str, counts: list[int]) -> str:
    """Rewrite the numeric cells of a markdown table row, preserving
    cell padding, bold markers, and any trailing columns."""
    prefix = line[: len(line) - len(line.lstrip())]
    body = line.strip()
    lead = "|" if body.startswith("|") else ""
    trail = "|" if body.endswith("|") else ""
    cells = body.strip("|").split("|")
    for i, value in enumerate(counts, start=1):
        if i >= len(cells):
            break
        # Replace the first run of digits in the cell, keeping the
        # surrounding whitespace and any `**` bold markers intact.
        cells[i] = re.sub(r"\d+", str(value), cells[i], count=1)
    return f"{prefix}{lead}{'|'.join(cells)}{trail}"


def check_category_table(
    path: Path,
    heading: str,
    category_labels: dict[str, str],
    category_counts: "Counter[str]",
    category_severity: dict[str, "Counter[str]"],
    category_autofix: "Counter[str]",
    totals: list[int],
    errors: list[str],
    fix: bool,
) -> None:
    """Verify (or rewrite) a per-category Rules/HIGH/MEDIUM/LOW/Auto-Fix table."""
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    stripped = [line.rstrip("\n") for line in lines]
    start, end = find_table(stripped, heading)
    table = parse_category_table(stripped[start:end])

    wanted: dict[str, list[int]] = {}
    for category, label in category_labels.items():
        wanted[label] = [
            category_counts.get(category, 0),
            category_severity[category].get("HIGH", 0),
            category_severity[category].get("MEDIUM", 0),
            category_severity[category].get("LOW", 0),
            category_autofix.get(category, 0),
        ]
    wanted["TOTAL"] = totals

    missing = [label for label in category_labels.values() if label not in table]
    for label in missing:
        errors.append(f"{path}: missing row for '{label}'")

    stale: dict[str, list[int]] = {}
    for label, expected in wanted.items():
        found = table.get(label)
        if found is None:
            if label == "TOTAL":
                errors.append(f"{path}: TOTAL expected {expected}, found {found}")
            continue
        if found != expected:
            stale[label] = expected

    if not stale:
        return
    if not fix:
        for label, expected in stale.items():
            if label == "TOTAL":
                errors.append(
                    f"{path}: TOTAL expected {expected}, found {table.get(label)}"
                )
            else:
                errors.append(
                    f"{path}: '{label}' expected {expected}, found {table.get(label)}"
                )
        return

    for idx in range(start, end):
        row = stripped[idx]
        cells = [cell.strip() for cell in row.strip().strip("|").split("|")]
        if len(cells) < 6:
            continue
        label = re.sub(r"[`*]", "", cells[0]).strip()
        if label in stale:
            lines[idx] = set_row_counts(row, stale[label]) + "\n"
    path.write_text("".join(lines), encoding="utf-8")
    print(f"[fixed] {path}: {heading} rows {sorted(stale)}")


def main(fix: bool = False) -> int:
    errors: list[str] = []
    data = json.loads(RULES_PATH.read_text(encoding="utf-8"))
    rules = data.get("rules", [])
    total = len(rules)

    severity_counts = Counter(rule["severity"] for rule in rules)
    category_counts: Counter[str] = Counter(rule["category"] for rule in rules)
    category_severity: dict[str, Counter[str]] = defaultdict(Counter)
    for rule in rules:
        category_severity[rule["category"]][rule["severity"]] += 1

    category_autofix: Counter[str] = Counter(
        rule["category"]
        for rule in rules
        if rule.get("fix", {}).get("autofix") is True
    )

    expected_total = total
    expected_high = severity_counts.get("HIGH", 0)
    expected_medium = severity_counts.get("MEDIUM", 0)
    expected_low = severity_counts.get("LOW", 0)
    expected_autofix = sum(category_autofix.values())

    category_labels = {
        "agent-skills": "Agent Skills",
        "claude-skills": "Claude Skills",
        "claude-hooks": "Claude Hooks",
        "claude-agents": "Claude Agents",
        "claude-memory": "Claude Memory",
        "claude-output-styles": "Claude Output Styles",
        "agents-md": "AGENTS.md",
        "claude-plugins": "Claude Plugins",
        "claude-settings": "Claude Settings",
        "copilot": "GitHub Copilot",
        "copilot-skills": "Copilot Skills",
        "mcp": "MCP",
        "xml": "XML",
        "references": "References",
        "prompt-engineering": "Prompt Eng",
        "cross-platform": "Cross-Platform",
        "cursor": "Cursor",
        "cursor-skills": "Cursor Skills",
        "cline": "Cline",
        "cline-skills": "Cline Skills",
        "opencode": "OpenCode",
        "opencode-skills": "OpenCode Skills",
        "gemini-agents": "Gemini Agents",
        "gemini-cli": "Gemini CLI",
        "codex": "Codex CLI",
        "codex-skills": "Codex Skills",
        "version-awareness": "Version Awareness",
        "windsurf": "Windsurf",
        "windsurf-skills": "Windsurf Skills",
        "kiro-agents": "Kiro Agents",
        "kiro-hooks": "Kiro Hooks",
        "kiro-mcp": "Kiro MCP",
        "kiro-powers": "Kiro Powers",
        "kiro-settings": "Kiro Settings",
        "kiro-skills": "Kiro Skills",
        "kiro-steering": "Kiro Steering",
        "amp-skills": "Amp Skills",
        "amp-checks": "Amp Checks",
        "roo-code": "Roo Code",
        "roo-code-skills": "Roo Code Skills",
    }

    totals = [
        expected_total,
        expected_high,
        expected_medium,
        expected_low,
        expected_autofix,
    ]
    for table_path, heading in (
        (ROOT / "knowledge-base" / "INDEX.md", "Validation Rules by Category"),
        (ROOT / "knowledge-base" / "VALIDATION-RULES.md", "Rule Count Summary"),
    ):
        check_category_table(
            table_path,
            heading,
            category_labels,
            category_counts,
            category_severity,
            category_autofix,
            totals,
            errors,
            fix,
        )

    spec_path = ROOT / "SPEC.md"
    spec_raw = spec_path.read_text(encoding="utf-8").splitlines(keepends=True)
    spec_lines = [line.rstrip("\n") for line in spec_raw]
    spec_start, spec_end = find_table(spec_lines, "What agnix Validates")
    spec_table = parse_spec_table(spec_lines[spec_start:spec_end])
    spec_map = {
        "Skills": ["agent-skills", "claude-skills"],
        "Hooks": ["claude-hooks"],
        "Memory (Claude Code)": ["claude-memory"],
        "Instructions (Cross-Tool)": ["agents-md"],
        "Agents": ["claude-agents"],
        "Plugins": ["claude-plugins"],
        "Claude Output Styles": ["claude-output-styles"],
        "Claude Settings": ["claude-settings"],
        "Prompt Engineering": ["prompt-engineering"],
        "Cross-Platform": ["cross-platform"],
        "MCP": ["mcp"],
        "XML": ["xml"],
        "References": ["references"],
        "GitHub Copilot": ["copilot"],
        "Cursor Project Rules": ["cursor"],
        "Cline": ["cline"],
        "OpenCode": ["opencode"],
        "Gemini Agents": ["gemini-agents"],
        "Gemini CLI": ["gemini-cli"],
        "Codex CLI": ["codex"],
        "Version Awareness": ["version-awareness"],
        "Cursor Skills": ["cursor-skills"],
        "Cline Skills": ["cline-skills"],
        "Copilot Skills": ["copilot-skills"],
        "Codex Skills": ["codex-skills"],
        "OpenCode Skills": ["opencode-skills"],
        "Windsurf": ["windsurf"],
        "Windsurf Skills": ["windsurf-skills"],
        "Kiro Agents": ["kiro-agents"],
        "Kiro Hooks": ["kiro-hooks"],
        "Kiro MCP": ["kiro-mcp"],
        "Kiro Powers": ["kiro-powers"],
        "Kiro Settings": ["kiro-settings"],
        "Kiro Steering": ["kiro-steering"],
        "Kiro Skills": ["kiro-skills"],
        "Amp Skills": ["amp-skills"],
        "Amp Checks": ["amp-checks"],
        "Roo Code Skills": ["roo-code-skills"],
        "Roo Code": ["roo-code"],
    }
    spec_sum = 0
    spec_stale: dict[str, int] = {}
    for label, categories in spec_map.items():
        expected = sum(category_counts.get(category, 0) for category in categories)
        found = spec_table.get(label)
        if found is None:
            errors.append(f"{spec_path}: missing row for '{label}'")
            continue
        if found != expected:
            if fix:
                spec_stale[label] = expected
            else:
                errors.append(
                    f"{spec_path}: '{label}' expected {expected}, found {found}"
                )
        spec_sum += expected
    if spec_sum != expected_total:
        errors.append(
            f"{spec_path}: expected total {expected_total}, summed {spec_sum}"
        )

    if spec_stale:
        # The count is the third column (Type | Files | Rules), so pad
        # the leading cells that set_row_counts would otherwise rewrite.
        for idx in range(spec_start, spec_end):
            cells = [c.strip() for c in spec_lines[idx].strip().strip("|").split("|")]
            if len(cells) < 3:
                continue
            label = cells[0]
            if label in spec_stale:
                row = spec_lines[idx]
                body = row.strip()
                parts = body.strip("|").split("|")
                parts[2] = re.sub(r"\d+", str(spec_stale[label]), parts[2], count=1)
                lead = "|" if body.startswith("|") else ""
                trail = "|" if body.endswith("|") else ""
                prefix = row[: len(row) - len(row.lstrip())]
                spec_raw[idx] = f"{prefix}{lead}{'|'.join(parts)}{trail}\n"
        spec_path.write_text("".join(spec_raw), encoding="utf-8")
        print(f"[fixed] {spec_path}: rows {sorted(spec_stale)}")

    require_counts_in_text(
        ROOT / "CLAUDE.md",
        r"knowledge-base/\s+#\s*(\d+)\s+rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "CLAUDE.md",
        r"(\d+)\s+rules defined in `knowledge-base/rules.json`",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "AGENTS.md",
        r"knowledge-base/\s+#\s*(\d+)\s+rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "AGENTS.md",
        r"(\d+)\s+rules defined in `knowledge-base/rules.json`",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "knowledge-base" / "README.md",
        r"VALIDATION-RULES.md\) - (\d+) rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "knowledge-base" / "INDEX.md",
        r"(\d+) validation rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "knowledge-base" / "INDEX.md",
        r"VALIDATION-RULES.md\) - (\d+) rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "knowledge-base" / "VALIDATION-RULES.md",
        r"Total Coverage.*?(\d+) validation rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "editors" / "vscode" / "README.md",
        r"Validates (\d+) rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "crates" / "agnix-lsp" / "README.md",
        r"Supports all agnix validation rules \((\d+) rules\)",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "docs" / "EDITOR-SETUP.md",
        r"- (\d+) validation rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "website" / "docs" / "intro.md",
        r"(\d+) rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "website" / "docs" / "intro.md",
        r"(\d+) validation rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "website" / "docs" / "getting-started.md",
        r"all (\d+) rules",
        expected_total,
        errors,
        fix,
    )
    require_counts_in_text(
        ROOT / "website" / "docs" / "contributing.md",
        r"validates against (\d+) rules",
        expected_total,
        errors,
        fix,
    )

    if errors:
        print("Rule count checks failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    # `--fix` rewrites the stale counts in place instead of reporting
    # them. scripts/sync-rule-bookkeeping.js invokes it that way so a
    # rule addition doesn't require hand-patching a dozen files. Rows
    # or patterns that are missing entirely are still hard errors:
    # inserting a new table row needs a human to place it.
    raise SystemExit(main(fix="--fix" in sys.argv[1:]))
