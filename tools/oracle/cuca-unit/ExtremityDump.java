import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import net.atmp.SvgOption;
import net.sourceforge.plantuml.FileFormat;
import net.sourceforge.plantuml.klimt.UStroke;
import net.sourceforge.plantuml.klimt.color.HColor;
import net.sourceforge.plantuml.klimt.color.HColorSet;
import net.sourceforge.plantuml.klimt.color.HColors;
import net.sourceforge.plantuml.klimt.drawing.UGraphic;
import net.sourceforge.plantuml.klimt.drawing.debug.StringBounderDebug;
import net.sourceforge.plantuml.klimt.drawing.debug.UGraphicDebug;
import net.sourceforge.plantuml.klimt.drawing.svg.UGraphicSvg;
import net.sourceforge.plantuml.klimt.geom.XDimension2D;
import net.sourceforge.plantuml.klimt.geom.XPoint2D;
import net.sourceforge.plantuml.svek.extremity.Extremity;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactory;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryArrow;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryArrowAndCircle;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryCircle;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryCircleConnect;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryCircleCross;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryCircleCrowfoot;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryCircleLine;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryCrowfoot;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryDiamond;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryDoubleLine;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryExtendsLike;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryHalfArrow;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryLineCrowfoot;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryNotNavigable;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryParenthesis;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryPlus;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactorySquare;
import net.sourceforge.plantuml.svek.extremity.ExtremityFactoryTriangle;

/**
 * Dumps every link-end decoration LinkDecor.getExtremityFactoryComplete can return, drawn the way SmetanaEdge
 * draws them (createUDrawable at a point and angle, no side), for the exact replay test in
 * crates/rockuml/src/svek/extremity/tests.rs. Each case lists getDecorationLength and the shapes UGraphicDebug
 * records; arcs, which only SVG draws differently from full ellipses, are also listed as SVG paths. Inputs are
 * written as their 16 hex digit IEEE bits.
 */
public class ExtremityDump {
	private static final HColor LINE = HColorSet.instance().getColorOrWhite("#181818");
	private static final HColor BACKGROUND = HColorSet.instance().getColorOrWhite("#F1F1F1");

	private static final XPoint2D[] POINTS = { new XPoint2D(0, 0), new XPoint2D(10.5, 20.25),
			new XPoint2D(-3.7, 123.456), new XPoint2D(100.123456789, -50.987654321) };

	private static final double[] ANGLES = { 0, Math.PI / 2, Math.PI, 3 * Math.PI / 2, 2 * Math.PI, -Math.PI / 2,
			-Math.PI, Math.PI / 2 + 0.0008, Math.PI / 2 + 0.001, Math.PI - 0.0007, 0.3, 1.234, 2.5, 4.0, 5.5, -2.0,
			-0.7, Math.atan2(-3, 4) + Math.PI };

	private static final String[] VARIANTS = { "hollow", "filled", "thick" };

	private final PrintStream out;

	private ExtremityDump(PrintStream out) {
		this.out = out;
	}

	public static void main(String[] args) throws Exception {
		try (PrintStream out = new PrintStream(args[0], "UTF-8")) {
			new ExtremityDump(out).run();
		}
	}

	private static Map<String, ExtremityFactory> factories() {
		final Map<String, ExtremityFactory> result = new LinkedHashMap<>();
		result.put("Plus", new ExtremityFactoryPlus(BACKGROUND));
		result.put("ExtendsLike(false)", new ExtremityFactoryExtendsLike(BACKGROUND, false));
		result.put("ExtendsLike(true)", new ExtremityFactoryExtendsLike(BACKGROUND, true));
		result.put("HalfArrow(1)", new ExtremityFactoryHalfArrow(1));
		result.put("HalfArrow(-1)", new ExtremityFactoryHalfArrow(-1));
		result.put("Triangle(8,3,8)", new ExtremityFactoryTriangle(null, 8, 3, 8));
		result.put("Triangle(18,6,18)", new ExtremityFactoryTriangle(null, 18, 6, 18));
		result.put("Crowfoot", new ExtremityFactoryCrowfoot());
		result.put("CircleCrowfoot", new ExtremityFactoryCircleCrowfoot());
		result.put("LineCrowfoot", new ExtremityFactoryLineCrowfoot());
		result.put("CircleLine", new ExtremityFactoryCircleLine());
		result.put("DoubleLine", new ExtremityFactoryDoubleLine());
		result.put("CircleCross", new ExtremityFactoryCircleCross(BACKGROUND));
		result.put("Arrow", new ExtremityFactoryArrow());
		result.put("ArrowAndCircle", new ExtremityFactoryArrowAndCircle(BACKGROUND));
		result.put("NotNavigable", new ExtremityFactoryNotNavigable());
		result.put("Diamond(false)", new ExtremityFactoryDiamond(false));
		result.put("Diamond(true)", new ExtremityFactoryDiamond(true));
		result.put("Circle(false)", new ExtremityFactoryCircle(false, BACKGROUND));
		result.put("Circle(true)", new ExtremityFactoryCircle(true, BACKGROUND));
		result.put("Square", new ExtremityFactorySquare(BACKGROUND));
		result.put("Parenthesis", new ExtremityFactoryParenthesis());
		result.put("CircleConnect", new ExtremityFactoryCircleConnect(BACKGROUND));
		return result;
	}

	private void run() throws IOException {
		out.println("# Generated by tools/oracle/cuca-unit/extremity.sh from ExtremityDump.java; do not edit.");
		for (Map.Entry<String, ExtremityFactory> entry : factories().entrySet())
			for (int i = 0; i < ANGLES.length; i++)
				debugCase(entry.getKey(), entry.getValue(), POINTS[i % POINTS.length], ANGLES[i],
						VARIANTS[i % VARIANTS.length]);

		for (String name : new String[] { "Parenthesis", "CircleConnect" })
			for (int i = 0; i < ANGLES.length; i++)
				svgCase(name, factories().get(name), POINTS[i % POINTS.length], ANGLES[i]);
	}

	private void debugCase(String name, ExtremityFactory factory, XPoint2D p0, double angle, String variant)
			throws IOException {
		final UGraphicDebug root = new UGraphicDebug(1.0, new XDimension2D(0, 0), null, null, 0, "none");
		UGraphic ug = root.apply(HColors.none().bg()).apply(LINE);
		if (variant.equals("filled"))
			ug = ug.apply(LINE.bg());
		else if (variant.equals("thick"))
			ug = ug.apply(UStroke.withThickness(2.5));

		final Extremity extremity = (Extremity) factory.createUDrawable(p0, angle, null);
		extremity.drawU(ug);

		out.println("case " + name + " " + bits(p0.getX()) + " " + bits(p0.getY()) + " " + bits(angle) + " "
				+ variant);
		out.println("decorationLength: " + extremity.getDecorationLength());
		final ByteArrayOutputStream document = new ByteArrayOutputStream();
		root.writeToStream(document, null, 96);
		final String text = document.toString("UTF-8");
		// Only the shapes: the header ends at the first blank line.
		out.print(text.substring(text.indexOf("\n\n") + 2));
		out.println("end");
	}

	private void svgCase(String name, ExtremityFactory factory, XPoint2D p0, double angle) throws IOException {
		final UGraphicSvg svg = UGraphicSvg.build(SvgOption.basic().withBackcolor(HColors.WHITE), false, 0, new StringBounderDebug(),
				FileFormat.SVG);
		final UGraphic ug = svg.apply(HColors.none().bg()).apply(LINE);
		factory.createUDrawable(p0, angle, null).drawU(ug);

		out.println("svg " + name + " " + bits(p0.getX()) + " " + bits(p0.getY()) + " " + bits(angle));
		final ByteArrayOutputStream document = new ByteArrayOutputStream();
		svg.writeToStream(document, null, 96);
		final Matcher paths = Pattern.compile("<path[^>]*/>").matcher(document.toString("UTF-8"));
		while (paths.find())
			out.println(paths.group());
		out.println("end");
	}

	private static String bits(double value) {
		return String.format("%016x", Double.doubleToRawLongBits(value));
	}
}
