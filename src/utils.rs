use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use ssh2::{CheckResult, ExtendedData, KnownHosts, PublicKey, Session};
use std::{
    collections::HashMap,
    io::{self, stdout, Read, Write},
};

pub fn check_hosts(known_hosts: &KnownHosts, hosts: &[&str], key: &[u8]) -> CheckResult {
    for host in hosts {
        let res = known_hosts.check(host, key);

        if !matches!(res, CheckResult::NotFound) {
            return res;
        }
    }

    CheckResult::NotFound
}

pub fn get_identities() -> Result<HashMap<Vec<u8>, PublicKey>> {
    let session = Session::new()?;

    let mut agent = session.agent()?;
    agent.connect()?;
    agent.list_identities()?;

    let mut keys = HashMap::new();

    for identity in agent.identities()? {
        keys.insert(Sha256::digest(identity.blob()).as_slice().into(), identity);
    }

    Ok(keys)
}

pub trait SessionExt {
    fn run(&mut self, cmd: &str, stdin: &[u8], return_output: bool) -> Result<String>;
}

impl SessionExt for Session {
    fn run(&mut self, cmd: &str, stdin: &[u8], return_output: bool) -> Result<String> {
        let mut channel = self.channel_session()?;
        channel.handle_extended_data(ExtendedData::Merge)?;

        channel.exec(cmd)?;
        if !stdin.is_empty() {
            channel.write_all(stdin)?;
        }
        channel.send_eof()?;

        let mut output = String::new();

        if return_output {
            channel.read_to_string(&mut output)?;
        } else {
            io::copy(&mut channel, &mut stdout())?;
        }

        channel.wait_close()?;
        channel.close()?;

        let status = channel.exit_status()?;
        if status != 0 {
            if return_output {
                bail!(
                    "`{}` returned exit status {}\noutput: {}",
                    cmd,
                    status,
                    output
                );
            }

            bail!("`{}` returned exit status {}", cmd, status);
        }

        Ok(output.trim().to_string())
    }
}
