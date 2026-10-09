"""Writes the feature status table: what rockuml ports of a PlantUML release, what comes next and what never will.

Usage: python tools/features/status.py [--check]
  Reads features/plantuml-<version>.json (made by inventory.py), features/decisions.json and rockuml's sources,
  and writes FEATURES.md and the website's page. --check only reports whether they are up to date.

Something is ported when rockuml names it: commands by their Java class name in the Rust modules of their
diagram (outside the stubs of not-ported commands), built-in functions by their %name, flags and output
formats by the command line's own tables. decisions.json says what is planned, and why some never will be.
"""

import json
import os
import re
import sys
from collections import Counter

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ENGINE = os.path.join(ROOT, "crates", "rockuml", "src")
CLI = os.path.join(ROOT, "crates", "rockuml-cli", "src")
SITE = "https://h3ll5ur7er.github.io/rockuml/"
STUB = re.compile(r'unported::\w+\(\s*"(\w+)"')

STATUSES = {
    "ported": ("✅", "Ported"),
    "partial": ("🟡", "Partly ported"),
    "soon": ("🔜", "Planned soon"),
    "later": ("🗓️", "Planned later"),
    "never": ("⛔", "Not planned"),
}


def read(path):
    with open(path, encoding="utf-8") as file:
        return file.read()


def rust_files(directory):
    files = {}
    for folder, _, names in os.walk(directory):
        for name in names:
            if name.endswith(".rs"):
                path = os.path.join(folder, name)
                files[os.path.relpath(path, directory).replace(os.sep, "/")] = read(path)
    return files


class Rockuml:
    """What rockuml's sources say they port."""

    def __init__(self):
        self.engine = rust_files(ENGINE)
        everything = "\n".join(self.engine.values())
        self.builtins = set(re.findall(r'"(%\w+)"', everything))
        self.flags = {
            flag: support
            for flag, support in re.findall(
                r'^\s*\w+: "(-[^"]+)" \[[^\]]*\] \w+ (Ported|Ignored|NotPorted|Dropped)\b',
                read(os.path.join(CLI, "cli_flag.rs")),
                re.M,
            )
        }
        formats = read(os.path.join(CLI, "file_format.rs"))
        enum = re.search(r"enum FileFormat \{(.*?)\}", formats, re.S).group(1)
        self.formats = {
            normalised(variant) for variant in re.findall(r"^\s*(\w+),", enum, re.M)
        } | {normalised(java) for java in re.findall(r"PlantUML's\s*(?:///\s*)?`(\w+)`", enum)}
        names = read(os.path.join(ENGINE, "style", "names.rs"))
        self.style_spellings = set(re.findall(r'= "(\w+)"', names))

    def commands_in(self, modules):
        """The Java class names the modules name, and those they only stub as not ported."""
        texts = [text for path, text in self.engine.items() if any(inside(path, module) for module in modules)]
        stubbed = {name for text in texts for name in STUB.findall(text)}
        named = {name for text in texts for name in re.findall(r"\b[A-Z]\w+\b", text)}
        return named - stubbed, stubbed


def inside(path, module):
    return path == module or path.startswith(module.rstrip("/") + "/")


def normalised(name):
    return name.replace("_", "").lower()


def corpus_counts():
    """Per corpus folder: its cases, and those whose shapes match PlantUML's (the parity test's `debug` kind)."""
    corpus = os.path.join(ROOT, "tests", "corpus")
    cases = {folder: sum(name.endswith(".puml") for name in os.listdir(os.path.join(corpus, folder))) for folder in os.listdir(corpus)}
    passing = Counter(
        line.split()[1].split("/")[0]
        for line in read(os.path.join(ROOT, "tests", "parity-passing.txt")).splitlines()
        if line.startswith("debug ")
    )
    return cases, passing


class Status:
    """The statuses of everything in a PlantUML inventory."""

    def __init__(self, inventory, decisions, rockuml):
        self.inventory = inventory
        self.decisions = decisions
        self.rockuml = rockuml
        self.diagrams = [self.diagram(entry) for entry in inventory["diagrams"]]
        self.builtins = [self.builtin(entry) for entry in inventory["builtins"]]
        self.flags = [self.flag(entry) for entry in inventory["cliFlags"]]
        self.formats = [self.format(name) for name in inventory["outputFormats"]]
        spellings = rockuml.style_spellings
        self.missing_style_names = [name for name in inventory["styleNames"] if name not in spellings]
        self.missing_style_properties = [name for name in inventory["styleProperties"] if name not in spellings]

    def diagram(self, entry):
        decision = self.decisions["diagrams"].get(entry["factory"])
        if decision is None:
            raise SystemExit(f"decisions.json says nothing about {entry['factory']}: add it to diagrams")
        result = {"factory": entry["factory"], **decision, "missing": [], "commands": len(entry["commands"])}
        if "rust" not in decision:
            return result
        named, stubbed = self.rockuml.commands_in(decision["rust"])
        for command in entry["commands"]:
            java = command["id"].split(".")[0]
            if java in named:
                continue
            # A stub recognises the command and says it is not ported; without one it is a syntax error.
            reported = "Recognised and reported as not ported." if java in stubbed else "Not recognised yet: a syntax error."
            decision_for_command = self.decisions["commands"].get(java, {"status": "soon"})
            reason = " ".join(filter(None, [decision_for_command.get("reason"), reported]))
            result["missing"].append({"id": command["id"], "status": decision_for_command["status"], "reason": reason})
        result["status"] = "partial" if result["missing"] else "ported"
        return result

    def builtin(self, entry):
        if entry["name"] in self.rockuml.builtins:
            return {**entry, "status": "ported"}
        return {**entry, **self.decisions["builtins"].get(entry["name"], self.decisions["builtins"]["_default"])}

    def flag(self, entry):
        support = self.rockuml.flags.get(entry["flag"])
        if support == "Ported":
            return {**entry, "status": "ported"}
        if support == "Ignored":
            return {**entry, "status": "ported", "reason": "Accepted without effect: it sets Java or Graphviz."}
        if support == "Dropped":
            return {**entry, "status": "never", "reason": self.decisions["cliFlags"]["_dropped"]}
        if support == "NotPorted":
            return {**entry, "status": "later", "reason": self.decisions["cliFlags"]["_notPorted"]}
        return {**entry, "status": "soon", "reason": "New in this PlantUML release: rockuml does not know it yet."}

    def format(self, name):
        if normalised(name) in self.rockuml.formats:
            return {"name": name, "status": "ported"}
        decisions = self.decisions["outputFormats"]
        return {"name": name, **decisions.get(name, decisions["_default"])}


def badge(status):
    symbol, label = STATUSES[status]
    return f"{symbol} {label}"


def render(status, link):
    """The page as Markdown; `link(slug)` is the address of a documentation page."""
    version = status.inventory["plantuml"]
    cases, passing = corpus_counts()
    diagrams = status.diagrams
    by_status = Counter(diagram["status"] for diagram in diagrams)
    lines = [tldr(status), ""]
    lines += [
        f"This page compares rockuml with PlantUML {version}, feature by feature: what is ported and ready to use, "
        "what is planned and what is not, with the reason. It is generated from PlantUML's own sources and from "
        "rockuml's, so it stays true as both evolve, and when PlantUML publishes a new release, comparing the two "
        "releases shows exactly what rockuml has to catch up on.",
        "",
        "| Status | Meaning |",
        "|---|---|",
        f"| {badge('ported')} | Works like in PlantUML. |",
        f"| {badge('partial')} | Works, except for the parts listed. |",
        f"| {badge('soon')} | Next on the plan, or a gap in something already ported. |",
        f"| {badge('later')} | Planned, after the more widely used features. |",
        f"| {badge('never')} | Not planned, for the reason given. |",
        "",
        "## At a glance",
        "",
        "| Area | rockuml |",
        "|---|---|",
        f"| Diagram types | {by_status['ported'] + by_status['partial']} of {len(diagrams)} ported, "
        f"{by_status['later'] + by_status['soon']} planned, {by_status['never']} not planned |",
        f"| Commands of the ported diagrams | {count_ported_commands(diagrams)} |",
        f"| Preprocessor functions | {count(status.builtins)} |",
        f"| Command line flags | {count(status.flags)} |",
        f"| Output formats | {count(status.formats)} |",
        f"| Style names and properties | {style_summary(status)} |",
        "",
        "## Diagram types",
        "",
        "| Diagram | Status | Reference diagrams | Notes | In PlantUML |",
        "|---|---|---|---|---|",
    ]
    order = {"ported": 0, "partial": 1, "soon": 2, "later": 3, "never": 4}
    commanded = [diagram for diagram in diagrams if diagram["commands"] and "rust" in diagram]
    shared = set.intersection(*({command["id"] for command in diagram["missing"]} for diagram in commanded))
    for diagram in sorted(diagrams, key=lambda diagram: order[diagram["status"]]):
        title = diagram["title"]
        if "docs" in diagram:
            title = f"[{title}]({link(diagram['docs'])})"
        reference = ""
        if "corpus" in diagram:
            total = sum(cases.get(folder, 0) for folder in diagram["corpus"])
            matching = sum(passing.get(folder, 0) for folder in diagram["corpus"])
            reference = f"{matching} of {total} match PlantUML"
        notes = diagram.get("notes") or diagram.get("reason", "")
        if diagram["missing"]:
            own = [command for command in diagram["missing"] if command["id"] not in shared]
            gaps = f"{len(diagram['missing'])} of {diagram['commands']} commands not yet ported, listed below"
            if len(own) < len(diagram["missing"]):
                gaps += f" ({len(diagram['missing']) - len(own)} of them shared by every diagram)"
            notes = (notes + " " if notes else "") + gaps + "."
        lines.append(f"| {title} | {badge(diagram['status'])} | {reference} | {notes} | `{diagram['factory']}` |")
    lines += [
        "",
        "Reference diagrams are the cases of rockuml's test corpus; a matching one gives the same shapes, at the same "
        "coordinates, as PlantUML.",
        "",
        "## Commands not yet ported",
        "",
        "Every command of a ported diagram type that is not listed here is ported. Commands are named after "
        "PlantUML's Java classes.",
        "",
    ]
    missing = Counter()
    where = {}
    for diagram in diagrams:
        for command in diagram["missing"]:
            missing[command["id"]] += 1
            where.setdefault(command["id"], (command, []))[1].append(diagram["title"])
    if missing:
        lines += ["| Command | Status | In | Notes |", "|---|---|---|---|"]
        for command_id in sorted(missing):
            command, titles = where[command_id]
            within = "every ported diagram" if len(titles) > 3 else ", ".join(titles)
            lines.append(f"| `{command_id}` | {badge(command['status'])} | {within} | {command.get('reason', '')} |")
    else:
        lines.append("None.")
    lines += section("Preprocessor functions", status.builtins, lambda entry: f"`{entry['name']}`")
    lines += section("Command line flags", status.flags, lambda entry: f"`{entry['flag']}`")
    lines += ["", "## Output formats", "", "| Format | Status | Notes |", "|---|---|---|"]
    for entry in sorted(status.formats, key=lambda entry: order[entry["status"]]):
        lines.append(f"| `{entry['name']}` | {badge(entry['status'])} | {entry.get('reason', '')} |")
    lines += [
        "",
        "## Style names and properties",
        "",
        style_details(status),
        "",
        "## When PlantUML releases a new version",
        "",
        "1. Make an inventory of the new release from its sources: "
        "`python tools/features/inventory.py <sources> <version> features/plantuml-<version>.json`.",
        f"2. Compare it with this one: `python tools/features/compare.py features/plantuml-{version}.json "
        "features/plantuml-<version>.json`. The report lists the new, removed and changed diagram types, commands, "
        "functions, flags, formats and style names, and says which of the changed ones rockuml has ported, since "
        "those need a second look.",
        "3. Port what is new, record decisions for the rest in `features/decisions.json`, set its `inventory` to "
        "the new file, and run `python tools/features/status.py` to update this page.",
        "",
    ]
    return "\n".join(lines)


def tldr(status):
    """A mind map of the diagram types, by status: the page's template, as every documentation page has one."""
    groups = {"ported": [], "planned": [], "never": []}
    # Single diagram types first, then groups such as the reference cards, each group once.
    for diagram in sorted(status.diagrams, key=lambda diagram: "group" in diagram):
        key = {"ported": "ported", "partial": "ported", "soon": "planned", "later": "planned"}.get(diagram["status"], "never")
        title = diagram.get("group") or re.sub(r"\s*\(.*\)$", "", diagram["title"]).replace("`", "")
        if title not in groups[key]:
            groups[key].append(title)
    planned = groups["planned"]
    lines = [
        "```rockuml tldr",
        "@startmindmap",
        "<style>",
        "mindmapDiagram {",
        "  .ported { BackgroundColor #BBF7D0 }",
        "  .planned { BackgroundColor #FEF3C7 }",
        "  .never { BackgroundColor #E5E7EB }",
        "}",
        "</style>",
        f"* rockuml and PlantUML {status.inventory['plantuml']}",
        f"** Ported: {len(groups['ported'])} diagram types <<ported>>",
    ]
    lines += [f"*** {title} <<ported>>" for title in groups["ported"]]
    lines += ["left side", "** Planned <<planned>>"]
    lines += [f"*** {title} <<planned>>" for title in planned[:8]]
    if len(planned) > 8:
        lines.append(f"*** and {len(planned) - 8} more <<planned>>")
    lines.append("** Not planned <<never>>")
    lines += [f"*** {title} <<never>>" for title in groups["never"]]
    lines += ["@endmindmap", "```"]
    return "\n".join(lines)


def count(entries):
    ported = sum(entry["status"] == "ported" for entry in entries)
    return f"{ported} of {len(entries)} ported"


def count_ported_commands(diagrams):
    ported = [diagram for diagram in diagrams if diagram["status"] in ("ported", "partial")]
    total = sum(diagram["commands"] for diagram in ported)
    missing = sum(len(diagram["missing"]) for diagram in ported)
    return f"{total - missing} of {total} ported"


def section(title, entries, name):
    lines = ["", f"## {title}", "", f"{count(entries)}. Not ported:", ""]
    pending = [entry for entry in entries if entry["status"] != "ported"]
    if not pending:
        return ["", f"## {title}", "", f"All {len(entries)} are ported."]
    lines += ["| Name | Status | Notes |", "|---|---|---|"]
    for entry in pending:
        lines.append(f"| {name(entry)} | {badge(entry['status'])} | {entry.get('reason', '')} |")
    return lines


def style_summary(status):
    total = len(status.inventory["styleNames"]) + len(status.inventory["styleProperties"])
    missing = len(status.missing_style_names) + len(status.missing_style_properties)
    return f"{total - missing} of {total} known"


def style_details(status):
    names, properties = status.inventory["styleNames"], status.inventory["styleProperties"]
    if not (status.missing_style_names or status.missing_style_properties):
        return (
            f"rockuml knows all {len(names)} style names (the selectors of `<style>` sheets) and all "
            f"{len(properties)} style properties of PlantUML."
        )
    unknown = ", ".join(f"`{name}`" for name in status.missing_style_names + status.missing_style_properties)
    return f"rockuml does not know these style names and properties yet: {unknown}."


def main(check):
    decisions = json.loads(read(os.path.join(ROOT, "features", "decisions.json")))
    inventory_path = os.path.join(ROOT, "features", decisions["inventory"])
    status = Status(json.loads(read(inventory_path)), decisions, Rockuml())
    pages = {
        os.path.join(ROOT, "FEATURES.md"): "# Feature status\n\n" + render(status, lambda slug: f"{SITE}docs/{slug}"),
        os.path.join(ROOT, "site", "projects", "site", "src", "app", "docs", "content", "features.md"): render(
            status, lambda slug: f"docs/{slug}"
        ),
    }
    stale = [path for path, text in pages.items() if not os.path.exists(path) or read(path) != text]
    if check:
        if stale:
            names = ", ".join(os.path.relpath(path, ROOT) for path in stale)
            raise SystemExit(f"Out of date: {names}. Run python tools/features/status.py and commit the result.")
        print("The feature status is up to date.")
        return
    for path in stale:
        with open(path, "w", encoding="utf-8", newline="\n") as file:
            file.write(pages[path])
    print(f"Wrote {len(stale)} file(s).")


if __name__ == "__main__":
    main("--check" in sys.argv[1:])
