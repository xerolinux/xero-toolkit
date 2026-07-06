//! Xero Authentication Client
//!
//! Command-line client for testing the authentication daemon.

use clap::Parser;
use std::sync::Arc;
use tokio::sync::Notify;
use xero_auth::shared::is_daemon_running;
use xero_auth::Client;

#[derive(Parser, Debug)]
#[command(name = "xero-auth")]
#[command(about = "Xero Authentication Client", long_about = None)]
struct Args {
    /// Environment variables to set (KEY=VALUE)
    #[arg(short, long)]
    env: Vec<String>,

    /// The program to execute
    program: String,

    /// Arguments for the program
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[tokio::main]
async fn main() {
    if !is_daemon_running() {
        eprintln!("Error: xero-auth daemon is not running");
        std::process::exit(1);
    }

    let mut args = Args::parse();

    let mut env_vars = Vec::new();
    for (key, value) in std::env::vars() {
        env_vars.push(format!("{}={}", key, value));
    }

    // Prepend inherited environment variables so explicit --env overrides them
    env_vars.extend(args.env);
    args.env = env_vars;

    let mut client = match Client::new().await {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Failed to connect to daemon: {}", e);
            std::process::exit(1);
        }
    };

    // Installing these handlers overrides the default terminate-immediately
    // disposition, so SIGTERM/SIGINT (sent by the GUI's Cancel button) instead
    // forward a Cancel to the daemon and wait for it to actually stop the
    // privileged command, rather than just killing this relay process and
    // leaving the real (root-owned) command running orphaned.
    let cancel = Arc::new(Notify::new());
    let cancel_for_signals = cancel.clone();
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("Failed to register SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => {}
            _ = sigint.recv() => {}
        }
        cancel_for_signals.notify_one();
    });

    let exit_code = match client
        .execute(
            &args.program,
            &args.args,
            args.env,
            None,
            |line| print!("{}", line),
            |line| eprint!("{}", line),
            cancel,
        )
        .await
    {
        Ok(code) => code,
        Err(e) => {
            eprintln!("Failed to execute command: {}", e);
            std::process::exit(1);
        }
    };

    std::process::exit(exit_code);
}
