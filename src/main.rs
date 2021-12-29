#![warn(clippy::pedantic)]

use anyhow::{bail, Result};
use clap::Parser;
use cli::Cli;
use once_cell::sync::OnceCell;
use serde::Deserialize;
use ssh2::{CheckResult, DisconnectCode, KnownHostFileKind, Session};
use std::{
    collections::HashMap,
    env, fs,
    io::{stdout, Write},
    net::{IpAddr, SocketAddr, TcpStream},
    path::PathBuf,
    process::Command,
};
use utils::{check_hosts, SessionExt};
use walkdir::WalkDir;

mod cli;
mod utils;

#[derive(Deserialize)]
struct Host {
    #[serde(rename = "host")]
    addr: IpAddr,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default = "default_user")]
    user: String,
}

impl Host {
    fn socket_addr(&self) -> SocketAddr {
        (self.addr, self.port).into()
    }
}

fn default_port() -> u16 {
    22
}

fn default_user() -> String {
    static USER: OnceCell<String> = OnceCell::new();

    USER.get_or_init(whoami::username).clone()
}

fn main() -> Result<()> {
    let mut args = Cli::parse();

    let flake_path = args.path.join("flake.nix");
    if !flake_path.exists() {
        bail!("No `flake.nix` found in this directory");
    }

    let hosts = Command::new("nix")
        .arg("eval")
        .arg("--json")
        .arg(format!("{}#deploy.hosts", args.path.to_str().unwrap()))
        .output()?;

    if !hosts.status.success() {
        bail!("Failed to get hosts ({})", String::from_utf8(hosts.stderr)?);
    }

    let hosts: HashMap<String, Host> = serde_json::from_slice(&hosts.stdout)?;

    if args.all {
        args.hosts = hosts.keys().cloned().collect();
    } else {
        for host in &args.hosts {
            if !hosts.contains_key(host) {
                bail!("Unknown host \"{}\"", host);
            }
        }
    }

    let mut archive_builder = tar::Builder::new(Vec::new());

    for entry in WalkDir::new(&args.path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !e.file_name().to_str().unwrap().starts_with('.'))
        .filter(|e| e.as_ref().unwrap().file_type().is_file())
    {
        archive_builder.append_path(entry?.path().strip_prefix(&args.path)?)?;
    }

    let archive = archive_builder.into_inner()?;

    let known_hosts_lines: Vec<String> =
        fs::read_to_string(PathBuf::from(env::var_os("HOME").unwrap()).join(".ssh/known_hosts"))?
            .lines()
            .map(String::from)
            .collect();

    for (hostname, host) in args.hosts.iter().map(|arg| (arg, hosts.get(arg).unwrap())) {
        if args.quiet {
            print!("deploying {}... ", hostname);
            stdout().flush()?;
        } else {
            println!("deploying {}...", hostname);
        }

        let socket_addr = host.socket_addr().to_string();

        let tcp = TcpStream::connect(&socket_addr)?;
        let mut ssh = Session::new()?;

        let mut known_hosts = ssh.known_hosts()?;

        for known_host in &known_hosts_lines {
            known_hosts.read_str(known_host, KnownHostFileKind::OpenSSH)?;
        }

        ssh.set_tcp_stream(tcp);
        ssh.handshake()?;

        let (host_key, _) = ssh.host_key().unwrap();

        match check_hosts(
            &known_hosts,
            &[&host.addr.to_string(), &socket_addr],
            host_key,
        ) {
            CheckResult::Match => {}
            CheckResult::Mismatch => bail!("known hosts mismatch!!!"),
            CheckResult::NotFound => {
                println!(
                    "host not found in known hosts (ssh at least once and compare fingerprints)"
                );

                continue;
            }
            CheckResult::Failure => bail!("couldn't check known hosts?"),
        }

        ssh.userauth_agent(&host.user)?;

        if !ssh.authenticated() {
            bail!("Authentication failed");
        }

        let tmpdir = PathBuf::from(ssh.run("mktemp -d", &[], false)?);

        ssh.run(&format!("tar -C {:?} -xf -", tmpdir), &archive, false)?;

        let rebuild_res = ssh.run(
            &format!("sudo nixos-rebuild {} --flake {:?} -L", args.action, tmpdir),
            &[],
            !args.quiet,
        );

        // SFTP_FXP_RMDIR may fail if the specified directory is not empty, so we just use `rm`
        ssh.run(&format!("rm -r {:?}", tmpdir), &[], false)?;

        rebuild_res?;

        ssh.disconnect(Some(DisconnectCode::ByApplication), "meow", None)?;

        if args.quiet {
            println!("done!");
        }
    }

    Ok(())
}
