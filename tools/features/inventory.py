"""Lists what a PlantUML release can do, from its Java sources, as JSON: the diagram factories and the commands
each one understands, the preprocessor's built-in functions, the command line flags, the output formats and the
style names. Comparing the inventories of two releases (compare.py) shows what a new release changed.

Usage: python tools/features/inventory.py SOURCES VERSION OUTPUT.json
  SOURCES is the folder holding net/sourceforge/plantuml, such as reference/plantuml-lgpl-1.2026.8-sources.

Each entry carries a hash of its Java source without comments and whitespace, so that a changed command, even
one whose name stayed, shows up in a comparison.
"""

import hashlib
import json
import os
import re
import sys

PACKAGE = os.path.join("net", "sourceforge", "plantuml")
LITERALS_AND_COMMENTS = re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|/\*.*?\*/|//[^\n]*', re.S)
LITERAL = re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'')
COMMAND_LIST_METHOD = re.compile(
    r"\bvoid\s+(\w+)\s*\(\s*(?:final\s+)?List<Command>\s+(\w+)\s*\)\s*\{"
)


class Sources:
    def __init__(self, root):
        self.root = os.path.join(root, PACKAGE)
        self.files = {}
        for directory, _, names in os.walk(self.root):
            for name in names:
                if name.endswith(".java"):
                    self.files.setdefault(name[:-5], os.path.join(directory, name))

    def code(self, class_name):
        """The class's source without comments, or "" for classes outside the tree."""
        path = self.files.get(class_name)
        if path is None:
            return ""
        with open(path, encoding="utf-8", errors="replace") as file:
            text = file.read()
        return LITERALS_AND_COMMENTS.sub(lambda match: match[0] if match[0][0] in "\"'" else " ", text)

    def digest(self, class_name):
        code = self.code(class_name)
        return digest(re.sub(r"\s+", "", code)) if code else None

    def relative(self, class_name):
        return os.path.relpath(self.files[class_name], self.root).replace(os.sep, "/")


def digest(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()[:16]


def after_closing(code, start):
    """The index after the bracket closing the one just before `start`, string literals skipped."""
    pairs = {"(": ")", "{": "}", "[": "]"}
    stack = [pairs[code[start - 1]]]
    index = start
    while stack:
        literal = LITERAL.match(code, index)
        if literal:
            index = literal.end()
            continue
        character = code[index]
        if character in pairs:
            stack.append(pairs[character])
        elif character == stack[-1]:
            stack.pop()
        index += 1
    return index


def command_list_methods(code):
    """The methods of a class that take a List<Command>, by name: the list's parameter name and the body."""
    return {
        match.group(1): (match.group(2), code[match.end() : after_closing(code, match.end()) - 1])
        for match in COMMAND_LIST_METHOD.finditer(code)
    }


def factories(sources):
    return re.findall(r"factories\.add\(new (\w+)\(", sources.code("PSystemBuilder"))


def commands_of(sources, factory):
    """The commands a diagram factory registers, in order and without repeats: `ClassName`, or
    `FactoryClass.createMethod` for those a command factory makes. Helpers such as CommonCommands are followed."""
    methods = command_list_methods(sources.code(factory))
    if "initCommandsList" not in methods:
        return []
    found = []
    expand(sources, factory, methods, *methods["initCommandsList"], found)
    return list(dict.fromkeys(found))


def expand(sources, owner, methods, commands, body, found):
    # Command factories held in variables: `F f = new F(...)` or the singleton `F f = F.ME`.
    declarations = re.findall(r"\b([A-Z]\w+)\s+(\w+)\s*=\s*(?:new\s+\1\s*\(|\1\.[A-Z][A-Z0-9_]*\s*;)", body)
    variables = {name: type_ for type_, name in declarations}
    index = 0
    statement = re.compile(rf"(\w+(?:\.\w+)?)\s*\(\s*{commands}\s*\)\s*;|{commands}\.add\(")
    while match := statement.search(body, index):
        if match.group(1):
            helper_class, _, method = match.group(1).rpartition(".")
            helper_class = helper_class or owner
            helper_methods = methods if helper_class == owner else command_list_methods(sources.code(helper_class))
            if method in helper_methods:
                expand(sources, helper_class, helper_methods, *helper_methods[method], found)
            index = match.end()
            continue
        end = after_closing(body, match.end())
        found.append(command_added(owner, body[match.end() : end - 1].strip(), variables))
        index = end


def command_added(owner, added, variables):
    """The command an expression given to cmds.add() stands for: `new X(...)` and the singleton `X.ME` are X;
    `new F(...).create(...)` and `f.create(...)`, with f a variable of type F, are `F.create`; a decorator such
    as `CommandDecoratorMultine.create(new CommandIf2(), 50)` stands for the command it wraps."""
    new = re.match(r"new\s+(\w+)\s*\(", added)
    if new:
        rest = added[after_closing(added, new.end()) :].strip()
        if not rest:
            return new.group(1)
        made = re.fullmatch(r"\.\s*(\w+)\s*\(.*\)", rest, re.S)
        if made:
            return f"{new.group(1)}.{made.group(1)}"
    call = re.fullmatch(r"(\w+)\s*\.\s*(\w+)\s*\((.*)\)", added, re.S)
    if call and call.group(1) in variables:
        return f"{variables[call.group(1)]}.{call.group(2)}"
    if call and call.group(1)[0].isupper():
        wrapped = re.match(r"\s*new\s+\w+\s*\(", call.group(3))
        if wrapped:
            arguments = call.group(3)
            return command_added(owner, arguments[: after_closing(arguments, wrapped.end())].strip(), variables)
        return f"{call.group(1)}.{call.group(2)}"
    singleton = re.fullmatch(r"([A-Z]\w+)\.[A-Z][A-Z0-9_]*", added)
    if singleton:
        return singleton.group(1)
    raise ValueError(f"{owner}: cannot read the command added by `{added}`")


def enum_constants(sources, class_name):
    """The constants of an enum, each with the text of its arguments."""
    code = sources.code(class_name)
    start = re.search(r"\benum\s+" + class_name + r"\b[^{]*\{", code).end()
    constants = []
    index = start
    while True:
        while code[index] in " \t\r\n,":
            index += 1
        annotation = re.match(r"@\w+\s*\(", code[index:])
        if annotation:
            index = after_closing(code, index + annotation.end())
            continue
        name = re.match(r"[A-Za-z_]\w*\b", code[index:])
        if not name:
            return constants
        index += name.end()
        arguments = ""
        if re.match(r"\s*\(", code[index:]):
            opening = index + code[index:].index("(") + 1
            index = after_closing(code, opening)
            arguments = re.sub(r"\s+", " ", code[opening : index - 1]).strip()
        if re.match(r"\s*\{", code[index:]):
            index = after_closing(code, index + code[index:].index("{") + 1)
        constants.append((name.group(0), arguments))


def builtins(sources):
    result = []
    for class_name in re.findall(r"addFunction\(new (\w+)\(", sources.code("TContext")):
        for name in sorted(set(re.findall(r'TFunctionSignature\("(%\w+)"', sources.code(class_name)))):
            result.append({"name": name, "class": class_name, "hash": sources.digest(class_name)})
    return result


def cli_flags(sources):
    flags = []
    for name, arguments in enum_constants(sources, "CliFlag"):
        flag = re.match(r'"([^"]+)"', arguments)
        if flag:
            flags.append({"name": name, "flag": flag.group(1), "hash": digest(arguments)})
    return flags


def main(root, version, output):
    sources = Sources(root)
    inventory = {
        "plantuml": version,
        "diagrams": [
            {
                "factory": factory,
                "file": sources.relative(factory),
                "hash": sources.digest(factory),
                "commands": [
                    {"id": command, "hash": sources.digest(command.split(".")[0])}
                    for command in commands_of(sources, factory)
                ],
            }
            for factory in factories(sources)
        ],
        "diagramTypes": [name for name, _ in enum_constants(sources, "DiagramType")],
        "builtins": builtins(sources),
        "cliFlags": cli_flags(sources),
        "outputFormats": [name for name, _ in enum_constants(sources, "FileFormat")],
        "styleNames": [name for name, _ in enum_constants(sources, "SName")],
        "styleProperties": [name for name, _ in enum_constants(sources, "PName")],
    }
    with open(output, "w", encoding="utf-8", newline="\n") as file:
        json.dump(inventory, file, indent=1, ensure_ascii=False)
        file.write("\n")
    commands = sum(len(diagram["commands"]) for diagram in inventory["diagrams"])
    print(
        f"PlantUML {version}: {len(inventory['diagrams'])} diagram factories, {commands} commands, "
        f"{len(inventory['builtins'])} built-in functions, {len(inventory['cliFlags'])} command line flags, "
        f"{len(inventory['outputFormats'])} output formats"
    )


if __name__ == "__main__":
    main(*sys.argv[1:])
