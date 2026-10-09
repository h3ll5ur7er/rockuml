"""Tests of the feature tools. Run: python -m unittest discover tools/features"""

import unittest

import compare
import inventory


class CommandsAdded(unittest.TestCase):
    def read(self, expression, variables=None):
        return inventory.command_added("Factory", expression, variables or {})

    def test_reads_a_command_class(self):
        self.assertEqual(self.read("new CommandTitle()"), "CommandTitle")
        self.assertEqual(self.read('new CommandHide("(hide|show)")'), "CommandHide")

    def test_reads_a_singleton(self):
        self.assertEqual(self.read("CommandTitle.ME"), "CommandTitle")
        self.assertEqual(self.read("CommandCreateElementMultilines.TYPE0"), "CommandCreateElementMultilines")

    def test_reads_what_a_command_factory_makes(self):
        self.assertEqual(self.read("new FactoryNote().createMultiLine(false)"), "FactoryNote.createMultiLine")
        self.assertEqual(
            self.read("notes.createSingleLine()", {"notes": "FactorySequenceNoteCommand"}),
            "FactorySequenceNoteCommand.createSingleLine",
        )

    def test_reads_the_command_a_decorator_wraps(self):
        self.assertEqual(self.read("CommandDecoratorMultine.create(new CommandIf2(), 50)"), "CommandIf2")

    def test_refuses_what_it_cannot_read(self):
        with self.assertRaises(ValueError):
            self.read("commands.get(0)")

    def test_follows_helpers_and_variables(self):
        found = []
        body = """
            CommonCommands.addTitles(cmds);
            final FactoryNote notes = new FactoryNote();
            cmds.add(notes.createSingleLine());
            cmds.add(new CommandArrow("->"));
        """

        class Sources:
            def code(self, class_name):
                return "static void addTitles(List<Command> cmds) { cmds.add(CommandTitle.ME); }"

        inventory.expand(Sources(), "Factory", {}, "cmds", body, found)
        self.assertEqual(found, ["CommandTitle", "FactoryNote.createSingleLine", "CommandArrow"])


class Comparison(unittest.TestCase):
    OLD = {
        "plantuml": "1.0",
        "diagrams": [
            {
                "factory": "SequenceDiagramFactory",
                "commands": [{"id": "CommandArrow", "hash": "a"}, {"id": "CommandOld", "hash": "b"}],
            }
        ],
        "builtins": [{"name": "%strlen", "hash": "s"}],
        "cliFlags": [{"flag": "--svg", "hash": "f"}],
        "outputFormats": ["PNG", "SVG"],
        "diagramTypes": ["SEQUENCE"],
        "styleNames": ["arrow"],
        "styleProperties": ["LineColor"],
    }
    NEW = {
        "plantuml": "2.0",
        "diagrams": [
            {
                "factory": "SequenceDiagramFactory",
                "commands": [{"id": "CommandArrow", "hash": "changed"}, {"id": "CommandNew", "hash": "c"}],
            },
            {"factory": "SoundDiagramFactory", "commands": []},
        ],
        "builtins": [{"name": "%strlen", "hash": "s"}, {"name": "%trim", "hash": "t"}],
        "cliFlags": [{"flag": "--svg", "hash": "f"}],
        "outputFormats": ["PNG", "SVG", "WEBP"],
        "diagramTypes": ["SEQUENCE", "SOUND"],
        "styleNames": ["arrow", "speaker"],
        "styleProperties": ["LineColor"],
    }

    def test_lists_what_is_new_removed_and_changed(self):
        text = compare.report(self.OLD, self.NEW, {"SequenceDiagramFactory": {"CommandArrow"}}, {"%strlen"})

        self.assertIn("# PlantUML 1.0 → 2.0", text)
        self.assertIn("**New:** `SoundDiagramFactory`", text)
        self.assertIn("**New commands:** `CommandNew`", text)
        self.assertIn("**Removed commands:** `CommandOld`", text)
        self.assertIn("**Changed commands:** `CommandArrow` (ported)", text)
        self.assertIn("**New:** `%trim`", text)
        self.assertIn("**New:** `WEBP`", text)
        self.assertIn("**New:** `speaker`", text)

    def test_says_when_nothing_changed(self):
        text = compare.report(self.OLD, self.OLD, {}, set())

        self.assertEqual(text.count("No changes."), 7)


if __name__ == "__main__":
    unittest.main()
