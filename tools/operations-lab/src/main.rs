//! `operations-lab server|client [options]`: see the crate README.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use operations_lab::client::LabClient;
use operations_lab::client_ui::ClientScreen;
use operations_lab::logbook::{LogBook, epoch_seconds};
use operations_lab::server::{Lab, LabServerConfig};
use operations_lab::server_ui::ServerScreen;
use operations_lab::tui;

const USAGE: &str = "\
usage:
  operations-lab server [--port 7447] [--proxy-port 7448] [--lease-secs 20]
                        [--observer-capacity 4] [--log PATH]
  operations-lab client [--connect tcp/127.0.0.1:7447 | --via-proxy]
                        [--timeout-ms 2000] [--log PATH]

Logs go to target/operations-lab/ unless --log is given.";

struct Args(Vec<String>);

impl Args {
    /// Removes `--name value` and returns the value.
    fn take(&mut self, name: &str) -> Result<Option<String>, String> {
        let Some(at) = self.0.iter().position(|a| a == name) else {
            return Ok(None);
        };
        if at + 1 >= self.0.len() {
            return Err(format!("{name} needs a value"));
        }
        let value = self.0.remove(at + 1);
        self.0.remove(at);
        Ok(Some(value))
    }

    fn take_parsed<T: std::str::FromStr>(&mut self, name: &str) -> Result<Option<T>, String> {
        self.take(name)?
            .map(|v| {
                v.parse()
                    .map_err(|_| format!("{name}: cannot read \"{v}\""))
            })
            .transpose()
    }

    fn flag(&mut self, name: &str) -> bool {
        let at = self.0.iter().position(|a| a == name);
        at.inspect(|&at| {
            self.0.remove(at);
        })
        .is_some()
    }

    fn finish(self) -> Result<(), String> {
        match self.0.first() {
            None => Ok(()),
            Some(extra) => Err(format!("unknown argument {extra}")),
        }
    }
}

fn log_book(path: Option<String>, default_name: &str) -> Result<Arc<LogBook>, String> {
    let path = path.map_or_else(
        || PathBuf::from("target/operations-lab").join(default_name),
        PathBuf::from,
    );
    LogBook::to_file(&path)
        .map(Arc::new)
        .map_err(|e| format!("cannot open the log {}: {e}", path.display()))
}

fn server(mut args: Args) -> Result<(), String> {
    let mut config = LabServerConfig::default();
    if let Some(port) = args.take_parsed("--port")? {
        config.port = port;
    }
    if let Some(port) = args.take_parsed("--proxy-port")? {
        config.proxy_port = port;
    }
    if let Some(secs) = args.take_parsed("--lease-secs")? {
        config.observe_lease = Duration::from_secs(secs);
    }
    if let Some(capacity) = args.take_parsed("--observer-capacity")? {
        config.observer_capacity = capacity;
    }
    let log = log_book(
        args.take("--log")?,
        &format!("server-{}.log", epoch_seconds()),
    )?;
    args.finish()?;
    let lab = Lab::start(config, log)?;
    let mut screen = ServerScreen::new(lab);
    let result = tui::run(&mut screen).map_err(|e| e.to_string());
    let lab = screen.into_lab();
    let path = lab.log().path().map(|p| p.display().to_string());
    lab.close();
    if let Some(path) = path {
        println!("server log: {path}");
    }
    result
}

fn client(mut args: Args) -> Result<(), String> {
    let via_proxy = args.flag("--via-proxy");
    let connect = args.take("--connect")?.unwrap_or_else(|| {
        if via_proxy {
            "tcp/127.0.0.1:7448".to_string()
        } else {
            "tcp/127.0.0.1:7447".to_string()
        }
    });
    let timeout = Duration::from_millis(args.take_parsed("--timeout-ms")?.unwrap_or(2000));
    let log = log_book(
        args.take("--log")?,
        &format!("client-{}-{}.log", epoch_seconds(), std::process::id()),
    )?;
    args.finish()?;
    let (client, notices) = LabClient::open(&connect, timeout, log)?;
    let mut screen = ClientScreen::new(client, notices);
    let result = tui::run(&mut screen).map_err(|e| e.to_string());
    let client = screen.into_client();
    let path = client.log().path().map(|p| p.display().to_string());
    client.close();
    if let Some(path) = path {
        println!("client log: {path}");
    }
    result
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let role = args.remove(0);
    let result = match role.as_str() {
        "server" => server(Args(args)),
        "client" => client(Args(args)),
        other => Err(format!("unknown role {other}\n\n{USAGE}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("operations-lab: {error}");
            ExitCode::FAILURE
        }
    }
}
