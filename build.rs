use clap::IntoApp;
use clap_generate::{
    generate_to,
    generators::{Bash, Fish, Zsh},
};
use std::{env, io::Result};

include!("src/cli.rs");

fn main() -> Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    let mut app = Cli::into_app();

    macro_rules! gen {
        ($shell:ident) => {
            generate_to($shell, &mut app, "deploy", &out_dir)?;
        };
    }

    gen!(Bash);
    gen!(Zsh);
    gen!(Fish);

    Ok(())
}
