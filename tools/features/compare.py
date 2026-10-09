"""Compares the inventories of two PlantUML releases (made by inventory.py), to show what a new release asks of
rockuml: what is new, what is gone, and what changed. Changed items that rockuml has ported are flagged, since
their port may need the same change.

Usage: python tools/features/compare.py OLD.json NEW.json [REPORT.md]
  Writes the report to REPORT.md, or prints it.
"""

import json
import sys

import status


def read(path):
    with open(path, encoding="utf-8") as file:
        return json.load(file)


def keyed(entries, key):
    return {entry[key]: entry for entry in entries}


def changes(old, new, key):
    """Added, removed and changed entries between two lists of dicts with a `hash`."""
    before, after = keyed(old, key), keyed(new, key)
    added = sorted(set(after) - set(before))
    removed = sorted(set(before) - set(after))
    changed = sorted(name for name in set(before) & set(after) if before[name]["hash"] != after[name]["hash"])
    return added, removed, changed


def report(old, new, ported_commands, ported_builtins):
    lines = [f"# PlantUML {old['plantuml']} → {new['plantuml']}", ""]

    def listing(title, names, ported=frozenset()):
        if names:
            lines.append(f"**{title}:** " + ", ".join(f"`{name}`" + (" (ported)" if name in ported else "") for name in names))
            lines.append("")

    old_diagrams, new_diagrams = keyed(old["diagrams"], "factory"), keyed(new["diagrams"], "factory")
    lines += ["## Diagram types", ""]
    listing("New", sorted(set(new_diagrams) - set(old_diagrams)))
    listing("Removed", sorted(set(old_diagrams) - set(new_diagrams)))
    for factory in sorted(set(old_diagrams) & set(new_diagrams)):
        command_added, command_removed, command_changed = changes(
            old_diagrams[factory]["commands"], new_diagrams[factory]["commands"], "id"
        )
        if command_added or command_removed or command_changed:
            ported = ported_commands.get(factory, set())
            lines += [f"### {factory}", ""]
            listing("New commands", command_added)
            listing("Removed commands", command_removed, ported)
            listing("Changed commands", command_changed, ported)
    for section, key, ported in [("Preprocessor functions", "name", ported_builtins), ("Command line flags", "flag", set())]:
        field = "builtins" if key == "name" else "cliFlags"
        entry_added, entry_removed, entry_changed = changes(old[field], new[field], key)
        lines += [f"## {section}", ""]
        listing("New", entry_added)
        listing("Removed", entry_removed, ported)
        listing("Changed", entry_changed, ported)
    for section, field in [
        ("Output formats", "outputFormats"),
        ("Diagram type names", "diagramTypes"),
        ("Style names", "styleNames"),
        ("Style properties", "styleProperties"),
    ]:
        lines += [f"## {section}", ""]
        listing("New", sorted(set(new[field]) - set(old[field])))
        listing("Removed", sorted(set(old[field]) - set(new[field])))
    return "\n".join(with_empty_sections_marked(lines)) + "\n"


def with_empty_sections_marked(lines):
    """Says "No changes." under a section heading followed directly by the next one, or by the end."""
    result = []
    for index, line in enumerate(lines):
        result.append(line)
        following = lines[index + 2] if index + 2 < len(lines) else None
        if line.startswith("## ") and (following is None or following.startswith("## ")):
            result += ["", "No changes."]
    return result


def main(old_path, new_path, output=None):
    old, new = read(old_path), read(new_path)
    current = status.Status(old, read(f"{status.ROOT}/features/decisions.json"), status.Rockuml())
    ported_commands = {
        diagram["factory"]: {
            command["id"] for command in keyed(old["diagrams"], "factory")[diagram["factory"]]["commands"]
        }
        - {command["id"] for command in diagram["missing"]}
        for diagram in current.diagrams
        if "rust" in diagram
    }
    ported_builtins = {entry["name"] for entry in current.builtins if entry["status"] == "ported"}
    text = report(old, new, ported_commands, ported_builtins)
    if output:
        with open(output, "w", encoding="utf-8", newline="\n") as file:
            file.write(text)
    else:
        sys.stdout.reconfigure(encoding="utf-8")
        print(text)


if __name__ == "__main__":
    main(*sys.argv[1:])
