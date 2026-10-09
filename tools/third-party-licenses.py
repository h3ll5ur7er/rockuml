"""Writes the licenses of the crates compiled into rockuml's binaries and wasm, for the release packages.

Usage: python3 tools/third-party-licenses.py OUTPUT.md

Only normal dependencies count: build and dev dependencies and procedural macros are not shipped. The texts
come from the crates' own sources, which `cargo metadata` makes cargo download.
"""

import json
import os
import subprocess
import sys

SHIPPED = ["rockuml-cli", "rockuml-wasm"]
LICENSE_PREFIXES = ("license", "licence", "copying", "notice", "unlicense")


def metadata():
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    return json.loads(result.stdout)


def is_proc_macro(package):
    return any("proc-macro" in target["kind"] for target in package["targets"])


def shipped_packages(data):
    """The ids of the packages the shipped crates need at run time, workspace crates excluded. Procedural
    macros run only while compiling, so neither they nor what they use end up in the binaries."""
    nodes = {node["id"]: node for node in data["resolve"]["nodes"]}
    packages = {package["id"]: package for package in data["packages"]}
    names = {id: package["name"] for id, package in packages.items()}
    workspace = set(data["workspace_members"])
    pending = [id for id in workspace if names[id] in SHIPPED]
    seen = set()
    while pending:
        id = pending.pop()
        if id in seen or is_proc_macro(packages[id]):
            continue
        seen.add(id)
        for dep in nodes[id]["deps"]:
            if any(kind["kind"] is None for kind in dep["dep_kinds"]):
                pending.append(dep["pkg"])
    return seen - workspace


def license_files(directory):
    return sorted(
        name
        for name in os.listdir(directory)
        if name.lower().startswith(LICENSE_PREFIXES) and os.path.isfile(os.path.join(directory, name))
    )


def main(output):
    data = metadata()
    packages = {package["id"]: package for package in data["packages"]}
    shipped = sorted(
        (packages[id] for id in shipped_packages(data)),
        key=lambda package: (package["name"], package["version"]),
    )
    lines = [
        "# Licenses of the crates in rockuml's binaries",
        "",
        "rockuml's binaries and wasm module contain these Rust crates, each under its own license, quoted",
        "below as the crate ships it.",
        "",
    ]
    missing = []
    for package in shipped:
        directory = os.path.dirname(package["manifest_path"])
        lines.append(f"## {package['name']} {package['version']}")
        lines.append("")
        lines.append(f"License: {package.get('license') or 'see below'}")
        if package.get("repository"):
            lines.append(f"Source: {package['repository']}")
        lines.append("")
        files = license_files(directory)
        if not files:
            missing.append(package["name"])
        for name in files:
            with open(os.path.join(directory, name), encoding="utf-8", errors="replace") as file:
                text = file.read().strip()
            lines.extend([f"### {name}", "", "```text", text, "```", ""])
    with open(output, "w", encoding="utf-8", newline="\n") as file:
        file.write("\n".join(lines))
    print(f"{len(shipped)} crates written to {output}.")
    if missing:
        print("Without license files of their own: " + ", ".join(missing))


if __name__ == "__main__":
    main(sys.argv[1])
