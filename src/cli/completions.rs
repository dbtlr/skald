use clap::CommandFactory;
use clap_complete::generate;
use std::io;

use super::root::Cli;

pub fn run(shell: clap_complete::Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "sk", &mut io::stdout());
}
