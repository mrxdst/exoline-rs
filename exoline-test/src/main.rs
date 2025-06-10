pub mod args;
pub mod connect;
mod util;

use std::process::ExitCode;

use args::Cli;
use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = connect::run(cli).await;

    if let Err(err) = result {
        eprintln!("Error: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
