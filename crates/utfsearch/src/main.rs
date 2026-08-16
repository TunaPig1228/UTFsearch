mod cli;
mod config;
mod mcp;

use clap::Parser;

fn main() -> std::process::ExitCode {
    cli::run(cli::Cli::parse())
}
