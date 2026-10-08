mod charset;
mod cli_flag;
mod cli_options;
mod cli_parsed;
mod console;
mod crash;
mod exit_status;
mod file_format;
mod file_group;
mod fonts;
mod help_print;
mod pico_web_server;
mod pipe;
mod run;
mod source_file_reader;
mod system_host;

use std::process::ExitCode;

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    std::thread::Builder::new()
        .stack_size(run::STACK_SIZE)
        .spawn(move || run::main(arguments))
        .expect("a thread can be started")
        .join()
        .map_or_else(
            |payload| {
                eprintln!("rockuml: crashed: {}", crash::message(payload.as_ref()));
                ExitCode::FAILURE
            },
            ExitCode::from,
        )
}
