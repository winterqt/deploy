use clap::{ArgEnum, Parser};
use std::{fmt::Display, path::PathBuf};

#[derive(Parser)]
#[clap(version, about = "Yet another NixOS deployment tool")]
pub struct Cli {
    #[clap(
        short,
        long,
        arg_enum,
        default_value = "switch",
        help = "nixos-rebuild action"
    )]
    pub action: RebuildAction,
    #[clap(short, long, default_value = ".", help = "configuration directory")]
    pub path: PathBuf,
    #[clap(short, long, help = "suppress build logs")]
    pub quiet: bool,
    #[clap(long, help = "deploy to all hosts")]
    pub all: bool,
    #[clap(required_unless_present = "all", help = "hosts to deploy to")]
    pub hosts: Vec<String>,
}

#[derive(ArgEnum, Clone, Copy)]
pub enum RebuildAction {
    Switch,
    Boot,
    Test,
}

impl Display for RebuildAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                RebuildAction::Switch => "switch",
                RebuildAction::Boot => "boot",
                RebuildAction::Test => "test",
            }
        )
    }
}
