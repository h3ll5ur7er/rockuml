package rockuml.oracle;

import java.lang.reflect.Field;
import java.util.Map;

import net.sourceforge.plantuml.FileFormat;
import net.sourceforge.plantuml.Run;
import net.sourceforge.plantuml.error.PSystemError;

/**
 * Runs PlantUML for golden generation.
 * <ul>
 * <li>Without the donation banners it adds to error images in some minutes of the hour, so that goldens do not
 * depend on when they were generated. rockuml never shows those banners.</li>
 * <li>With {@code -f svg-deterministic}: SVG measured with PlantUML's font-independent width table. The CLI has
 * no name for that format, because it shares the name "svg" with the font-measured one.</li>
 * </ul>
 */
public final class OracleMain {
	private OracleMain() {
	}

	public static void main(String[] args) throws Exception {
		PSystemError.disableTimeBasedErrorDecorations();
		registerDeterministicSvgFormat();
		Run.main(args);
	}

	@SuppressWarnings("unchecked")
	private static void registerDeterministicSvgFormat() throws ReflectiveOperationException {
		final Field byName = FileFormat.class.getDeclaredField("byName");
		byName.setAccessible(true);
		((Map<String, FileFormat>) byName.get(null)).put("svg-deterministic", FileFormat.SVG_DETERMINISTIC);
	}
}
