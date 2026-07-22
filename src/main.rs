//! bb-cli — a Bitbucket Cloud CLI.
//!
//! Rust rewrite of the original PHP tool. `main` stays thin: parse the CLI,
//! run preflight, dispatch to a command handler, and catch a single top-level
//! error to print in red — the same shape as the old `bin/bb` router.

mod cli;
mod client;
mod commands;
mod config;
mod error;
mod models;
mod output;
mod repo;

use clap::Parser;
use owo_colors::OwoColorize;

use cli::{Cli, Command};
use error::Result;

fn main() {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("{}", err.to_string().red());
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Api(args) => commands::api::run(args, &cli.global),
        Command::Auth(args) => commands::auth::run(args, &cli.global),
        Command::Branch(args) => commands::branch::run(args, &cli.global),
        Command::Browse(args) => commands::browse::run(args, &cli.global),
        Command::Env(args) => commands::env::run(args, &cli.global),
        Command::Pipeline(args) => commands::pipeline::run(args, &cli.global),
        Command::Pr(args) => commands::pr::run(args, &cli.global),
        Command::Repo(args) => commands::repo::run(args, &cli.global),
        Command::PrDetails(args) => commands::pr_details::run(args, &cli.global),
        Command::Upgrade(args) => commands::upgrade::run(args, &cli.global),
    }
}
