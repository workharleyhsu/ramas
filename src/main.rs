use anyhow::Result;
use clap::Parser;
use ramas::cli::Cli;

fn main() -> Result<()> {
    let _cli = Cli::parse();
    Ok(())
}
