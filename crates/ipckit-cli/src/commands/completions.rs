//! Shell completions command

use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

pub fn completions(shell: Shell) {
    let mut cmd = crate::Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}
