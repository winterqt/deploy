use anyhow::{bail, Result};
use ssh2::{CheckResult, ExtendedData, KnownHosts, Session};
use std::io::{self, stdout, Read, Write};

pub fn check_hosts(known_hosts: &KnownHosts, hosts: &[&str], key: &[u8]) -> CheckResult {
    for host in hosts {
        let res = known_hosts.check(host, key);

        if !matches!(res, CheckResult::NotFound) {
            return res;
        }
    }

    CheckResult::NotFound
}

pub trait SessionExt {
    fn run(&mut self, cmd: &str, stdin: &[u8], dual: bool) -> Result<String>;
}

impl SessionExt for Session {
    fn run(&mut self, cmd: &str, stdin: &[u8], dual: bool) -> Result<String> {
        let mut channel = self.channel_session()?;
        channel.handle_extended_data(ExtendedData::Merge)?;

        channel.exec(cmd)?;
        if !stdin.is_empty() {
            channel.write_all(stdin)?;
        }
        channel.send_eof()?;

        let mut output = String::new();

        if dual {
            let mut ov = Vec::new();
            let mut writer = DualWriter::new(&mut ov, stdout());
            io::copy(&mut channel, &mut writer)?;
            output = String::from_utf8(ov)?;
        } else {
            channel.read_to_string(&mut output)?;
        }

        channel.wait_close()?;
        channel.close()?;

        let status = channel.exit_status()?;
        if status != 0 {
            if dual {
                bail!("`{}` returned exit status {}", cmd, status);
            }

            bail!(
                "`{}` returned exit status {}\noutput: {}",
                cmd,
                status,
                output
            );
        }

        Ok(output.trim().to_string())
    }
}

struct DualWriter<A, B>
where
    A: Write,
    B: Write,
{
    a: A,
    b: B,
}

impl<A, B> DualWriter<A, B>
where
    A: Write,
    B: Write,
{
    fn new(a: A, b: B) -> DualWriter<A, B> {
        DualWriter { a, b }
    }
}

impl<A, B> Write for DualWriter<A, B>
where
    A: Write,
    B: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let a = self.a.write(buf)?;
        let b = self.b.write(buf)?;
        assert_eq!(a, b);
        Ok(a)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.a.flush()?;
        self.b.flush()?;
        Ok(())
    }
}
