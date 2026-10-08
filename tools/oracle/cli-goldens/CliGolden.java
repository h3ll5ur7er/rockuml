import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Paths;
import java.util.List;

/**
 * Runs the golden model's command line with the arguments listed in a file, one per line.
 * <p>
 * On Windows the {@code java} launcher expands {@code *} and {@code ?} in its arguments before PlantUML sees
 * them, which would hide PlantUML's own wildcard matching; arguments read from a file reach it verbatim.
 * {@code <WORKDIR>} in an argument stands for the current directory, for scenarios that need absolute paths.
 */
public final class CliGolden {
	private CliGolden() {
	}

	public static void main(String[] args) throws Exception {
		final String workdir = Paths.get("").toAbsolutePath().toString();
		final List<String> arguments = Files.readAllLines(Paths.get(args[0]), StandardCharsets.UTF_8);
		arguments.replaceAll(argument -> argument.replace("<WORKDIR>", workdir));
		rockuml.oracle.OracleMain.main(arguments.toArray(new String[0]));
	}
}
