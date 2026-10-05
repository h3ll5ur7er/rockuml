use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    if arguments.iter().any(|argument| argument == "--version") {
        println!(
            "rockuml {} (PlantUML {} compatible)",
            env!("CARGO_PKG_VERSION"),
            rockuml::PLANTUML_VERSION
        );
        return ExitCode::SUCCESS;
    }

    eprintln!("rockuml: unsupported arguments: {}", arguments.join(" "));
    ExitCode::FAILURE
}
