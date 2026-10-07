import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import java.util.List;

import net.sourceforge.plantuml.decoration.symbol.USymbol;
import net.sourceforge.plantuml.decoration.symbol.USymbols;
import net.sourceforge.plantuml.klimt.Fashion;
import net.sourceforge.plantuml.klimt.UStroke;
import net.sourceforge.plantuml.klimt.UTranslate;
import net.sourceforge.plantuml.klimt.awt.XColor;
import net.sourceforge.plantuml.klimt.color.HColor;
import net.sourceforge.plantuml.klimt.color.HColors;
import net.sourceforge.plantuml.klimt.drawing.UGraphic;
import net.sourceforge.plantuml.klimt.drawing.debug.UGraphicDebug;
import net.sourceforge.plantuml.klimt.font.FontConfiguration;
import net.sourceforge.plantuml.klimt.font.StringBounder;
import net.sourceforge.plantuml.klimt.font.UFontFactory;
import net.sourceforge.plantuml.klimt.geom.HorizontalAlignment;
import net.sourceforge.plantuml.klimt.geom.XDimension2D;
import net.sourceforge.plantuml.klimt.geom.XPoint2D;
import net.sourceforge.plantuml.klimt.shape.TextBlock;
import net.sourceforge.plantuml.klimt.shape.TextBlockEmpty;
import net.sourceforge.plantuml.klimt.shape.TextBlockRaw;
import net.sourceforge.plantuml.klimt.shape.UHorizontalLine;

/**
 * Dumps every USymbol's small and big forms, drawn on the debug UGraphic, for the exact comparison in
 * crates/rockuml/src/decoration/symbol/tests.rs. The Rust test rebuilds the same inputs from the case names, so the
 * two lists of cases below must stay in step with it.
 */
public class USymbolDump {

	private static final String[] CODES = { "ACTION", "ACTOR_AWESOME", "ACTOR_HOLLOW", "ACTOR_STICKMAN",
			"ACTOR_STICKMAN_BUSINESS", "AGENT", "ARCHIMATE", "ARTIFACT", "BOUNDARY", "CARD", "CLOUD", "COLLECTIONS",
			"COMPONENT_RECTANGLE", "COMPONENT1", "COMPONENT2", "CONTROL", "DATABASE", "ENTITY_DOMAIN", "FILE",
			"FOLDER", "FRAME", "GROUP", "HEXAGON", "INTERFACE", "LABEL", "NODE", "PACKAGE", "PARTITION", "PERSON",
			"PROCESS", "QUEUE", "RECTANGLE", "STACK", "STORAGE", "USECASE", "USECASE_BUSINESS" };

	private static final XPoint2D[] PROBES = { new XPoint2D(-1, -1), new XPoint2D(10, 5), new XPoint2D(40, -2),
			new XPoint2D(45, 0), new XPoint2D(50, -1), new XPoint2D(60, 10), new XPoint2D(100, -5),
			new XPoint2D(150, 20) };

	private static final StringBounder BOUNDER = new UGraphicDebug(1, new XDimension2D(0, 0), null, null, 0, "none")
			.getStringBounder();

	private final PrintStream out;

	private USymbolDump(PrintStream out) {
		this.out = out;
	}

	public static void main(String[] args) throws Exception {
		try (PrintStream out = new PrintStream(args[0], "UTF-8")) {
			new USymbolDump(out).run();
		}
	}

	private static FontConfiguration labelFont() {
		return FontConfiguration.blackBlueTrue(UFontFactory.serif(14));
	}

	private static FontConfiguration stereoFont() {
		return FontConfiguration.blackBlueTrue(UFontFactory.monospace(11));
	}

	private static TextBlock raw(FontConfiguration font, String... lines) {
		return new TextBlockRaw(Arrays.asList(lines), font);
	}

	private static TextBlock empty() {
		return new TextBlockEmpty(0, 0);
	}

	/** Two blocks with a separator between them, as bodies draw them. */
	private static TextBlock separated(TextBlock top, char style, TextBlock title, TextBlock bottom) {
		return new TextBlock() {
			public void drawU(UGraphic ug) {
				top.drawU(ug);
				final double y = top.calculateDimension(ug.getStringBounder()).getHeight();
				final UHorizontalLine line = title == null ? UHorizontalLine.infinite(1, 0, 0, style)
						: UHorizontalLine.infinite(1, 0, 0, title, style);
				ug.apply(UTranslate.dy(y)).draw(line);
				bottom.drawU(ug.apply(UTranslate.dy(y)));
			}

			public XDimension2D calculateDimension(StringBounder stringBounder) {
				return top.calculateDimension(stringBounder).mergeTB(bottom.calculateDimension(stringBounder));
			}
		};
	}

	private static HColor color(int rgb) {
		return HColors.simple(XColor.from(rgb));
	}

	private static Fashion fashion(String name) {
		final Fashion plain = new Fashion(color(0xFFEEDD), color(0x112233)).withStroke(UStroke.withThickness(1.5));
		switch (name) {
		case "round":
			return plain.withCorner(10, 0);
		case "diagonal":
			return plain.withCorner(0, 8);
		default:
			return plain;
		}
	}

	private static HorizontalAlignment alignment(String name) {
		return HorizontalAlignment.valueOf(name.toUpperCase());
	}

	// Each small case: name, label, stereotype, stereotype alignment, fashion.
	private TextBlock smallLabel(String kase) {
		switch (kase) {
		case "plain":
			return raw(labelFont(), "Label");
		case "stereo":
			return raw(labelFont(), "Some label", "x");
		case "dashes":
			return separated(raw(labelFont(), "Top line"), '.', null, raw(labelFont(), "Bottom"));
		case "titled":
			return separated(raw(labelFont(), "Above the line"), '=', raw(stereoFont(), "T"),
					raw(labelFont(), "Below"));
		case "tall":
			return raw(labelFont(), "First line of text", "Second", "Third line", "Fourth", "Fifth line here",
					"Sixth");
		default:
			throw new IllegalArgumentException(kase);
		}
	}

	private TextBlock smallStereo(String kase) {
		switch (kase) {
		case "stereo":
		case "titled":
			return raw(stereoFont(), "<<stereo>>");
		case "dashes":
			return raw(stereoFont(), "<<s>>");
		default:
			return empty();
		}
	}

	private static final List<String[]> SMALL_CASES = Arrays.asList( //
			new String[] { "plain", "center", "plain" }, //
			new String[] { "stereo", "right", "round" }, //
			new String[] { "dashes", "left", "diagonal" }, //
			new String[] { "titled", "center", "plain" }, //
			new String[] { "tall", "center", "round" });

	// Each big case: title, stereotype, width, height, label alignment, stereotype alignment, fashion.
	private static final List<String[]> BIG_CASES = Arrays.asList( //
			new String[] { "titled", "200", "120", "center", "center", "plain" }, //
			new String[] { "stereo", "160", "90", "left", "right", "round" }, //
			new String[] { "untitled", "80", "60", "right", "left", "diagonal" }, //
			new String[] { "narrow", "120", "150", "center", "center", "plain" });

	private TextBlock bigTitle(String kase) {
		switch (kase) {
		case "titled":
			return raw(labelFont(), "Package title");
		case "stereo":
			return raw(labelFont(), "T");
		case "narrow":
			return raw(labelFont(), "A long title text");
		default:
			return empty();
		}
	}

	private TextBlock bigStereo(String kase) {
		if (kase.equals("stereo"))
			return raw(stereoFont(), "<<big>>");
		return empty();
	}

	private void run() throws Exception {
		out.println("# Generated by tools/oracle/cuca-unit/usymbol.sh from USymbolDump.java; do not edit.");
		for (String code : CODES) {
			final USymbol symbol = (USymbol) USymbols.class.getField(code).get(null);
			for (String[] kase : SMALL_CASES) {
				final TextBlock block = symbol.asSmall(raw(labelFont(), "Name"), smallLabel(kase[0]),
						smallStereo(kase[0]), fashion(kase[2]), alignment(kase[1]));
				dump("small " + code + " " + String.join(" ", kase), block);
			}
			for (String[] kase : BIG_CASES) {
				final TextBlock block;
				try {
					block = symbol.asBig(bigTitle(kase[0]), alignment(kase[3]), bigStereo(kase[0]),
							Double.parseDouble(kase[1]), Double.parseDouble(kase[2]), fashion(kase[5]),
							alignment(kase[4]));
				} catch (UnsupportedOperationException e) {
					continue;
				}
				dump("big " + code + " " + String.join(" ", kase), block);
			}
		}
	}

	private void dump(String header, TextBlock block) throws Exception {
		out.println("=== " + header);
		final XDimension2D dim = block.calculateDimension(BOUNDER);
		out.println("dimension: " + dim.getWidth() + " " + dim.getHeight());
		for (XPoint2D probe : PROBES) {
			final UTranslate force = block.getMagneticBorder().getForceAt(BOUNDER, probe);
			if (force.getDx() != 0 || force.getDy() != 0)
				out.println("force " + probe.getX() + " " + probe.getY() + ": " + force.getDx() + " "
						+ force.getDy());
		}
		final UGraphicDebug ug = new UGraphicDebug(1, dim, null, null, 0, "none");
		block.drawU(ug);
		final ByteArrayOutputStream bytes = new ByteArrayOutputStream();
		ug.writeToStream(bytes, null, 96);
		final String[] lines = new String(bytes.toByteArray(), StandardCharsets.UTF_8).split("\n", -1);
		boolean inHeader = true;
		for (String line : lines) {
			if (inHeader) {
				inHeader = !line.isEmpty();
				continue;
			}
			// Shapes the debug output cannot describe are stamped with the time.
			if (line.startsWith("UGraphicDebug "))
				line = line.replaceFirst("^(UGraphicDebug \\S+) .*$", "$1 DATE");
			out.println(line);
		}
	}
}
