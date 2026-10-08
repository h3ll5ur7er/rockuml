import java.io.PrintStream;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;
import java.util.Locale;

import net.sourceforge.plantuml.BlockUml;
import net.sourceforge.plantuml.FileFormat;
import net.sourceforge.plantuml.SourceStringReader;
import net.sourceforge.plantuml.activitydiagram3.ActivityDiagram3;
import net.sourceforge.plantuml.activitydiagram3.Instruction;
import net.sourceforge.plantuml.activitydiagram3.ftile.Ftile;
import net.sourceforge.plantuml.activitydiagram3.ftile.FtileFactory;
import net.sourceforge.plantuml.activitydiagram3.ftile.FtileGeometry;
import net.sourceforge.plantuml.activitydiagram3.ftile.Swimlanes;
import net.sourceforge.plantuml.core.Diagram;
import net.sourceforge.plantuml.klimt.UTranslate;
import net.sourceforge.plantuml.klimt.font.StringBounder;

/**
 * Dumps the tile tree PlantUML builds for each activity diagram given: per tile its class, its geometry and
 * where its parent draws it, measured with the debug format's string bounder. A debugging aid for porting
 * tiles: run it with tools/oracle/activity-unit/ftiles.sh and compare with the Rust tree.
 */
public class FtileDump {

	private final PrintStream out;
	private StringBounder stringBounder;

	private FtileDump(PrintStream out) {
		this.out = out;
	}

	public static void main(String[] args) throws Exception {
		final FtileDump dump = new FtileDump(new PrintStream(System.out, true, "UTF-8"));
		for (int i = 1; i < args.length; i++)
			dump.run(args[0], args[i]);
	}

	private void run(String corpus, String file) throws Exception {
		final String text = new String(Files.readAllBytes(Paths.get(corpus, file)), StandardCharsets.UTF_8);
		final List<BlockUml> blocks = new SourceStringReader(text).getBlocks();
		final Diagram diagram = blocks.get(0).getDiagram();
		if (diagram instanceof ActivityDiagram3 == false)
			return;
		final Swimlanes swimlanes = (Swimlanes) field(diagram, ActivityDiagram3.class, "swimlanes");
		stringBounder = FileFormat.DEBUG.getDefaultStringBounder();
		final Method getFtileFactory = Swimlanes.class.getDeclaredMethod("getFtileFactory", StringBounder.class);
		getFtileFactory.setAccessible(true);
		final FtileFactory factory = (FtileFactory) getFtileFactory.invoke(swimlanes, stringBounder);
		final Instruction root = (Instruction) field(swimlanes, Swimlanes.class, "root");
		out.println("=== " + file);
		tile(root.createFtile(factory), null, 0);
	}

	private void tile(Ftile tile, Ftile parent, int depth) {
		final StringBuilder sb = new StringBuilder();
		for (int i = 0; i < depth; i++)
			sb.append("  ");
		sb.append(tile.getClass().getSimpleName());
		if (parent != null)
			sb.append(" at ").append(translate(parent, tile));
		sb.append(" ").append(geometry(tile.calculateDimension(stringBounder)));
		out.println(sb);
		try {
			for (Ftile child : tile.getMyChildren())
				tile(child, tile, depth + 1);
		} catch (UnsupportedOperationException e) {
			// Leaves without children say so by failing.
		}
	}

	private String translate(Ftile parent, Ftile child) {
		try {
			final UTranslate translate = parent.getTranslateFor(child, stringBounder);
			if (translate == null)
				return "null";
			return "(" + num(translate.getDx()) + "," + num(translate.getDy()) + ")";
		} catch (RuntimeException e) {
			return "?";
		}
	}

	private static String geometry(FtileGeometry geo) {
		final String out = geo.hasPointOut() ? num(geo.getOutY()) : "-";
		return "w=" + num(geo.getWidth()) + " h=" + num(geo.getHeight()) + " left=" + num(geo.getLeft()) + " inY="
				+ num(geo.getInY()) + " outY=" + out;
	}

	private static String num(double value) {
		return String.format(Locale.US, "%.4f", value);
	}

	private static Object field(Object object, Class<?> type, String name) throws Exception {
		final Field field = type.getDeclaredField(name);
		field.setAccessible(true);
		return field.get(object);
	}
}
