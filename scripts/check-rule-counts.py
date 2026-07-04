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


def extract_table(lines: list[str], heading: str) -> list[str]:
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
            return lines[start:end]
    raise ValueError(f"Heading '{heading}' not found")


def parse_category_table(table_lines: list[str]) -> dict[str, list[int]]:
    rows: dict[str, list[int]] = {}
    for line in table_lines:
        if not line.lstrip().startswith("|"):
            continue
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) < 5:
            continue
        label = re.sub(r"[`*]", "", cells[0]).strip()
        if label.lower() in {"category", "total rules"}:
            continue
        counts = [
            parse_int(cells[1]),
            parse_int(cells[2]),
            parse_int(cells[3]),
            parse_int(cells[4]),
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
    path: Path, pattern: str, expected: int, errors: list[str]
) -> None:
    text = path.read_text(encoding="utf-8")
    matches = re.findall(pattern, text)
    if not matches:
        errors.append(f"{path}: missing pattern '{pattern}'")
        return
    for match in matches:
        value = int(match)
        if value != expected:
            errors.append(
                f"{path}: expected {expected} for pattern '{pattern}', found {value}"
            )


def main() -> int:
    errors: list[str] = []
    data = json.loads(RULES_PATH.read_text(encoding="utf-8"))
    rules = data.get("rules", [])
    total = len(rules)

    severity_counts = Counter(rule["severity"] for rule in rules)
    category_counts: Counter[str] = Counter(rule["category"] for rule in rules)
    category_severity: dict[str, Counter[str]] = defaultdict(Counter)
    for rule in rules:
        category_severity[rule["category"]][rule["severity"]] += 1

    expected_total = total
    expected_high = severity_counts.get("HIGH", 0)
    expected_medium = severity_counts.get("MEDIUM", 0)
    expected_low = severity_counts.get("LOW", 0)

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

    index_path = ROOT / "knowledge-base" / "INDEX.md"
    index_lines = index_path.read_text(encoding="utf-8").splitlines()
    index_table = parse_category_table(
        extract_table(index_lines, "Validation Rules by Category")
    )
    for category, label in category_labels.items():
        counts = index_table.get(label)
        if counts is None:
            errors.append(f"{index_path}: missing row for '{label}'")
            continue
        expected = [
            category_counts.get(category, 0),
            category_severity[category].get("HIGH", 0),
            category_severity[category].get("MEDIUM", 0),
            category_severity[category].get("LOW", 0),
        ]
        if counts != expected:
            errors.append(
                f"{index_path}: '{label}' expected {expected}, found {counts}"
            )

    index_total = index_table.get("TOTAL")
    if index_total != [expected_total, expected_high, expected_medium, expected_low]:
        errors.append(
            f"{index_path}: TOTAL expected {[expected_total, expected_high, expected_medium, expected_low]}, "
            f"found {index_total}"
        )

    validation_path = ROOT / "knowledge-base" / "VALIDATION-RULES.md"
    validation_lines = validation_path.read_text(encoding="utf-8").splitlines()
    validation_table = parse_category_table(
        extract_table(validation_lines, "Rule Count Summary")
    )
    for category, label in category_labels.items():
        counts = validation_table.get(label)
        if counts is None:
            errors.append(f"{validation_path}: missing row for '{label}'")
            continue
        expected = [
            category_counts.get(category, 0),
            category_severity[category].get("HIGH", 0),
            category_severity[category].get("MEDIUM", 0),
            category_severity[category].get("LOW", 0),
        ]
        if counts != expected:
            errors.append(
                f"{validation_path}: '{label}' expected {expected}, found {counts}"
            )

    validation_total = validation_table.get("TOTAL")
    if validation_total != [
        expected_total,
        expected_high,
        expected_medium,
        expected_low,
    ]:
        errors.append(
            f"{validation_path}: TOTAL expected {[expected_total, expected_high, expected_medium, expected_low]}, "
            f"found {validation_total}"
        )

    spec_path = ROOT / "SPEC.md"
    spec_lines = spec_path.read_text(encoding="utf-8").splitlines()
    spec_table = parse_spec_table(extract_table(spec_lines, "What agnix Validates"))
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
    for label, categories in spec_map.items():
        expected = sum(category_counts.get(category, 0) for category in categories)
        found = spec_table.get(label)
        if found is None:
            errors.append(f"{spec_path}: missing row for '{label}'")
            continue
        if found != expected:
            errors.append(f"{spec_path}: '{label}' expected {expected}, found {found}")
        spec_sum += expected
    if spec_sum != expected_total:
        errors.append(
            f"{spec_path}: expected total {expected_total}, summed {spec_sum}"
        )

    require_counts_in_text(
        ROOT / "CLAUDE.md",
        r"knowledge-base/\s+#\s*(\d+)\s+rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "CLAUDE.md",
        r"(\d+)\s+rules defined in `knowledge-base/rules.json`",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "AGENTS.md",
        r"knowledge-base/\s+#\s*(\d+)\s+rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "AGENTS.md",
        r"(\d+)\s+rules defined in `knowledge-base/rules.json`",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "knowledge-base" / "README.md",
        r"VALIDATION-RULES.md\) - (\d+) rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "knowledge-base" / "INDEX.md",
        r"(\d+) validation rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "knowledge-base" / "INDEX.md",
        r"VALIDATION-RULES.md\) - (\d+) rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "knowledge-base" / "VALIDATION-RULES.md",
        r"Total Coverage.*?(\d+) validation rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "editors" / "vscode" / "README.md",
        r"Validates (\d+) rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "crates" / "agnix-lsp" / "README.md",
        r"Supports all agnix validation rules \((\d+) rules\)",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "docs" / "EDITOR-SETUP.md",
        r"- (\d+) validation rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "website" / "docs" / "intro.md",
        r"(\d+) rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "website" / "docs" / "intro.md",
        r"(\d+) validation rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "website" / "docs" / "getting-started.md",
        r"all (\d+) rules",
        expected_total,
        errors,
    )
    require_counts_in_text(
        ROOT / "website" / "docs" / "contributing.md",
        r"validates against (\d+) rules",
        expected_total,
        errors,
    )

    if errors:
        print("Rule count checks failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
