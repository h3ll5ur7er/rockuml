import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.Set;

import net.sourceforge.plantuml.SourceStringReader;
import net.sourceforge.plantuml.TitledDiagram;
import net.sourceforge.plantuml.activitydiagram3.PositionedNote;
import net.sourceforge.plantuml.activitydiagram3.ftile.AbstractFtile;
import net.sourceforge.plantuml.activitydiagram3.ftile.Ftile;
import net.sourceforge.plantuml.activitydiagram3.ftile.FtileGeometry;
import net.sourceforge.plantuml.activitydiagram3.ftile.Swimlane;
import net.sourceforge.plantuml.activitydiagram3.ftile.TextBlockInterceptorUDrawable;
import net.sourceforge.plantuml.activitydiagram3.ftile.vcompact.FtileGroup;
import net.sourceforge.plantuml.activitydiagram3.ftile.vcompact.FtileNoteAlone;
import net.sourceforge.plantuml.activitydiagram3.ftile.vcompact.FtileWithNoteOpale;
import net.sourceforge.plantuml.activitydiagram3.ftile.vcompact.FtileWithNotes;
import net.sourceforge.plantuml.decoration.symbol.USymbol;
import net.sourceforge.plantuml.decoration.symbol.USymbols;
import net.sourceforge.plantuml.klimt.color.ColorType;
import net.sourceforge.plantuml.klimt.color.Colors;
import net.sourceforge.plantuml.klimt.color.HColor;
import net.sourceforge.plantuml.klimt.color.HColors;
import net.sourceforge.plantuml.klimt.creole.Display;
import net.sourceforge.plantuml.klimt.drawing.UGraphic;
import net.sourceforge.plantuml.klimt.drawing.debug.UGraphicDebug;
import net.sourceforge.plantuml.klimt.font.StringBounder;
import net.sourceforge.plantuml.klimt.geom.VerticalAlignment;
import net.sourceforge.plantuml.klimt.geom.XDimension2D;
import net.sourceforge.plantuml.klimt.shape.URectangle;
import net.sourceforge.plantuml.sequencediagram.NotePosition;
import net.sourceforge.plantuml.sequencediagram.NoteType;
import net.sourceforge.plantuml.style.ISkinParam;
import net.sourceforge.plantuml.style.Style;
import net.sourceforge.plantuml.svek.UGraphicForSnake;

/**
 * Dumps tiles with notes and groups around a plain box, their geometry and their drawing on the debug UGraphic,
 * for the exact comparison in crates/rockuml/src/diagram/activity3/tests/notes_groups.rs. The Rust test builds
 * the same tiles from the case lines, so the cases below must stay in step with it.
 *
 * A case is a name, the skin lines of its diagram, then what is built, one item a line:
 * <ul>
 * <li>{@code box W H OVERFLOW}: a box W by H that draws OVERFLOW wider than it says (the innermost tile);</li>
 * <li>{@code note SIDE TYPE COLOR|line|line}: a note (SIDE left or right, TYPE note or floating, COLOR - or a
 * back colour);</li>
 * <li>{@code opale ALIGN}: the notes so far around the tile, as FtileWithNoteOpale.create with a link;</li>
 * <li>{@code notes ALIGN}: the notes so far around the tile, as FtileWithNotes;</li>
 * <li>{@code alone}: the first note so far alone, as FtileNoteAlone;</li>
 * <li>{@code group TYPE COLOR|title}: the tile in a group (TYPE as written in the diagram, COLOR - or a back
 * colour).</li>
 * </ul>
 * Building notes clears the notes so far.
 */
public class NoteGroupDump {

	private static final StringBounder BOUNDER = new UGraphicDebug(1, new XDimension2D(0, 0), null, null, 0, "none")
			.getStringBounder();

	static final String[][] CASES = { //
			{ "right", "", "box 120 36 0", "note right note -|Checks types and ranges", "opale CENTER" }, //
			{ "left-creole", "", "box 120 36 0",
					"note left note -|This note is on several|//lines// and can|contain <b>HTML</b>|====|* Calling the method \"\"foo()\"\" is prohibited",
					"opale CENTER" }, //
			{ "floating", "", "box 120 36 0", "note right floating -|Aggregation runs|every five minutes",
					"opale CENTER" }, //
			{ "colored", "", "box 80 20 0", "note right note #lightgreen|Automatic", "opale CENTER" }, //
			{ "tall-top", "", "box 60 10 0", "note left note -|one|two|three", "opale TOP" }, //
			{ "styled", "<style>|note {|  BackGroundColor lightblue|  LineColor red|  LineThickness 2|  MaximumWidth 60|}|</style>",
					"box 120 36 0", "note right note -|A long note that should wrap over several lines",
					"opale CENTER" }, //
			{ "aligned", "skinparam noteTextAlignment center", "box 120 36 0",
					"note left note -|centred|short and a longer line", "opale CENTER" }, //
			{ "several", "", "box 100 30 0", "note left note -|left one", "note right note -|right one",
					"note right note #pink|right two|second line", "opale CENTER" }, //
			{ "notes-top", "", "box 100 80 0", "note left note -|on top", "notes TOP" }, //
			{ "alone", "", "note right note -|Alone|with two lines", "alone" }, //
			{ "alone-floating", "skinparam noteFontColor blue", "note left floating #red|Floating alone", "alone" }, //
			{ "partition", "", "box 120 36 0", "group partition -|Initialization" }, //
			{ "group", "", "box 120 36 0", "group group -|Running" }, //
			{ "package", "", "box 120 36 0", "group package -|Data layer" }, //
			{ "rectangle", "", "box 120 36 0", "group rectangle -|Business layer" }, //
			{ "card", "", "box 120 36 0", "group card -|Presentation layer" }, //
			{ "wide-title", "", "box 40 20 0", "group partition -|A partition with a title much wider than its box" }, //
			{ "colored-group", "", "box 120 36 0", "group partition #lightblue|Order handling" }, //
			{ "overflow", "", "box 100 30 45", "group partition -|Overflow" }, //
			{ "nested", "", "box 120 36 0", "group partition #lightyellow|Payment",
					"group partition #lightblue|Order handling" }, //
			{ "styled-group",
					"<style>|partition {|  RoundCorner 12|  LineThickness 2|  FontSize 18|}|</style>|skinparam packageTitleAlignment left",
					"box 120 36 0", "group partition -|Styled" }, //
			{ "note-in-group", "", "box 120 36 0", "note right note -|Inside", "opale CENTER",
					"group group -|With a note" } };

	private final PrintStream out;

	private NoteGroupDump(PrintStream out) {
		this.out = out;
	}

	public static void main(String[] args) throws Exception {
		try (PrintStream out = new PrintStream(args[0], "UTF-8")) {
			new NoteGroupDump(out).run();
		}
	}

	/** A box that may draw wider than it says, as tiles with arrow labels do. */
	static class Box extends AbstractFtile {

		private final double width;
		private final double height;
		private final double overflow;

		Box(ISkinParam skinParam, double width, double height, double overflow) {
			super(skinParam);
			this.width = width;
			this.height = height;
			this.overflow = overflow;
		}

		@Override
		protected FtileGeometry calculateDimensionFtile(StringBounder stringBounder) {
			return new FtileGeometry(width, height, width / 2, 0, height);
		}

		public void drawU(UGraphic ug) {
			ug.draw(URectangle.build(width + overflow, height));
		}

		@Override
		public Collection<Ftile> getMyChildren() {
			return Collections.emptyList();
		}

		public Set<Swimlane> getSwimlanes() {
			return Collections.emptySet();
		}

		public Swimlane getSwimlaneIn() {
			return null;
		}

		public Swimlane getSwimlaneOut() {
			return null;
		}
	}

	private static ISkinParam skinParam(String skin) {
		final StringBuilder source = new StringBuilder("@startuml\n");
		if (skin.isEmpty() == false)
			for (String line : skin.split("\\|"))
				source.append(line).append('\n');
		source.append("start\n@enduml\n");
		return ((TitledDiagram) new SourceStringReader(source.toString()).getBlocks().get(0).getDiagram())
				.getSkinParam();
	}

	private void run() throws Exception {
		out.println("# Generated by tools/oracle/activity-unit/note-group.sh from NoteGroupDump.java; do not edit.");
		for (String[] kase : CASES)
			dump(kase);
	}

	private void dump(String[] kase) throws Exception {
		final ISkinParam skinParam = skinParam(kase[1]);
		Ftile tile = null;
		final List<PositionedNote> notes = new ArrayList<>();
		for (String item : Arrays.copyOfRange(kase, 2, kase.length)) {
			final String[] words = item.split("\\|", 2)[0].split(" ");
			final String rest = item.contains("|") ? item.split("\\|", 2)[1] : "";
			switch (words[0]) {
			case "box":
				tile = new Box(skinParam, Double.parseDouble(words[1]), Double.parseDouble(words[2]),
						Double.parseDouble(words[3]));
				break;
			case "note":
				notes.add(new PositionedNote(Display.create(Arrays.asList(rest.split("\\|"))),
						NotePosition.valueOf(words[1].toUpperCase()),
						words[2].equals("floating") ? NoteType.FLOATING_NOTE : NoteType.NOTE, null,
						colors(skinParam, words[3]), null));
				break;
			case "opale":
				tile = FtileWithNoteOpale.create(tile, notes, true, VerticalAlignment.valueOf(words[1]));
				notes.clear();
				break;
			case "notes":
				tile = new FtileWithNotes(tile, notes, VerticalAlignment.valueOf(words[1]));
				notes.clear();
				break;
			case "alone":
				final PositionedNote note = notes.get(0);
				final ISkinParam muted = note.getColors().mute(skinParam);
				tile = new FtileNoteAlone(muted.shadowing(null), note.getDisplay(), muted,
						note.getType() == NoteType.NOTE, null);
				notes.clear();
				break;
			case "group":
				final USymbol symbol = symbol(words[1]);
				final Style style = FtileGroup.getStyleSignature(symbol)
						.getMergedStyle(skinParam.getCurrentStyleBuilder());
				final Colors back = colors(skinParam, words[2]);
				tile = new FtileGroup(tile, Display.getWithNewlines(skinParam.getPragma(), rest),
						back.getColor(ColorType.BACK), skinParam, symbol, style);
				break;
			default:
				throw new IllegalArgumentException(item);
			}
		}
		out.println("=== " + kase[0]);
		final FtileGeometry geometry = tile.calculateDimension(BOUNDER);
		out.println("geometry: " + geometry.getWidth() + " " + geometry.getHeight() + " " + geometry.getLeft() + " "
				+ geometry.getInY() + " " + geometry.getOutY() + " " + geometry.hasPointOut());
		final UGraphicDebug ug = new UGraphicDebug(1, new XDimension2D(geometry.getWidth(), geometry.getHeight()),
				null, null, 0, "none");
		new TextBlockInterceptorUDrawable(tile, HColors.BLACK, false).drawU(new UGraphicForSnake(ug));
		final ByteArrayOutputStream bytes = new ByteArrayOutputStream();
		ug.writeToStream(bytes, null, 96);
		final String[] lines = new String(bytes.toByteArray(), StandardCharsets.UTF_8).split("\n", -1);
		boolean inHeader = true;
		for (String line : lines) {
			if (inHeader) {
				inHeader = !line.isEmpty();
				continue;
			}
			// The shapes it does not describe carry the time of drawing.
			out.println(line.replaceFirst("^(UGraphicDebug \\S+) .*$", "$1 DATE"));
		}
	}

	/** As CommandPartition3 reads the group's type. */
	private static USymbol symbol(String type) {
		switch (type) {
		case "package":
			return USymbols.PACKAGE;
		case "rectangle":
			return USymbols.RECTANGLE;
		case "card":
			return USymbols.CARD;
		case "group":
			return USymbols.GROUP;
		default:
			return USymbols.PARTITION;
		}
	}

	private static Colors colors(ISkinParam skinParam, String color) throws Exception {
		if (color.equals("-"))
			return Colors.empty();
		return new Colors(color, skinParam.getIHtmlColorSet(), ColorType.BACK);
	}
}
