package rockuml.oracle;

import net.sourceforge.plantuml.Run;
import net.sourceforge.plantuml.error.PSystemError;

/**
 * Runs PlantUML without the donation banners it adds to error images in some minutes of the hour, so that
 * goldens do not depend on when they were generated. rockuml never shows those banners.
 */
public final class OracleMain {
	private OracleMain() {
	}

	public static void main(String[] args) throws Exception {
		PSystemError.disableTimeBasedErrorDecorations();
		Run.main(args);
	}
}
