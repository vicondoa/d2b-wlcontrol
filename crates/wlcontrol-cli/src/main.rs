//! `nixling-wlcontrol` — Waybar module, control-center launcher, and action
//! dispatcher for nixling VMs.
//!
//! Owning wave: **Wave 0 (integrator) skeleton**; **Wave 2 — CLI/action shell
//! agent** hardens the Waybar loop (signals, no-overlap refresh, backoff),
//! single-instance `open`, and the full action surface.

use std::io::Write as _;
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};
use wlcontrol_core::model::ActionKind;
use wlcontrol_core::{plan, reduce, Config, PlannedAction, WlState};
use wlcontrol_nixling::NixlingClient;

/// Starter Waybar config snippet, kept in sync with `data/`.
const WAYBAR_CONFIG_SNIPPET: &str = include_str!("../../../data/waybar-module.jsonc");
/// Starter CSS, kept in sync with `data/`.
const STYLE_SNIPPET: &str = include_str!("../../../data/style.css");

#[derive(Debug, Parser)]
#[command(
    name = "nixling-wlcontrol",
    version,
    about = "Clean Waybar indicator and control center for nixling VMs."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the continuous Waybar custom JSON module.
    Waybar,
    /// Open (or focus) the GTK control center.
    Open,
    /// Print the normalized control-surface state as JSON.
    StatusJson,
    /// Dispatch a single control action.
    Action {
        #[command(subcommand)]
        action: ActionCommand,
    },
    /// Print a starter Waybar custom-module config snippet.
    PrintWaybarConfig,
    /// Print a starter CSS snippet.
    PrintCss,
}

#[derive(Debug, Subcommand)]
enum ActionCommand {
    /// Refresh state (used by Waybar middle-click).
    Refresh,
    /// Cycle the Waybar compact/detail display mode.
    CycleDisplay,
    /// Start a VM.
    Start { vm: String },
    /// Stop a VM.
    Stop { vm: String },
    /// Restart a VM.
    Restart { vm: String },
    /// Activate a VM's current closure.
    Switch { vm: String },
    /// Launch a terminal running an interactive guest shell.
    Terminal { vm: String },
    /// Attach a USB busid to a VM.
    UsbAttach { vm: String, bus_id: String },
    /// Detach a USB busid from a VM.
    UsbDetach { vm: String, bus_id: String },
    /// Verify a VM's store live pool.
    StoreVerify { vm: String },
}

impl ActionCommand {
    fn into_kind(self) -> ActionKind {
        match self {
            ActionCommand::Refresh => ActionKind::Refresh,
            ActionCommand::CycleDisplay => ActionKind::CycleDisplay,
            ActionCommand::Start { vm } => ActionKind::Start { vm },
            ActionCommand::Stop { vm } => ActionKind::Stop { vm },
            ActionCommand::Restart { vm } => ActionKind::Restart { vm },
            ActionCommand::Switch { vm } => ActionKind::Switch { vm },
            ActionCommand::Terminal { vm } => ActionKind::LaunchTerminal { vm },
            ActionCommand::UsbAttach { vm, bus_id } => ActionKind::UsbAttach { vm, bus_id },
            ActionCommand::UsbDetach { vm, bus_id } => ActionKind::UsbDetach { vm, bus_id },
            ActionCommand::StoreVerify { vm } => ActionKind::StoreVerify { vm },
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("nixling-wlcontrol: {err}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> wlcontrol_core::WlResult<ExitCode> {
    let config = Config::load()?;
    match cli.command {
        Command::Waybar => run_waybar(&config),
        Command::Open => run_open(&config),
        Command::StatusJson => run_status_json(&config),
        Command::Action { action } => run_action(&config, action.into_kind()),
        Command::PrintWaybarConfig => {
            print!("{WAYBAR_CONFIG_SNIPPET}");
            Ok(ExitCode::SUCCESS)
        }
        Command::PrintCss => {
            print!("{STYLE_SNIPPET}");
            Ok(ExitCode::SUCCESS)
        }
    }
}

/// Build the current reduced state from one refresh cycle.
fn current_state(config: &Config) -> WlState {
    let client = NixlingClient::new(config);
    reduce::reduce(client.refresh())
}

fn run_status_json(config: &Config) -> wlcontrol_core::WlResult<ExitCode> {
    let state = current_state(config);
    let mut json = serde_json::to_string_pretty(&state)?;
    json.push('\n');
    print!("{json}");
    Ok(ExitCode::SUCCESS)
}

/// Continuous Waybar custom-module loop.
///
/// Wave 2 hardens this with signal-driven refresh, non-overlapping refresh,
/// and daemon-down backoff. The Wave 0 baseline polls on a fixed cadence.
fn run_waybar(config: &Config) -> wlcontrol_core::WlResult<ExitCode> {
    let interval = Duration::from_millis(config.refresh_interval_ms.max(250));
    let mut stdout = std::io::stdout();
    loop {
        let state = current_state(config);
        let line = wlcontrol_waybar::render(&state);
        // Best-effort write; a closed pipe means Waybar went away.
        if stdout.write_all(line.to_json_line().as_bytes()).is_err() {
            break;
        }
        let _ = stdout.flush();
        thread::sleep(interval);
    }
    Ok(ExitCode::SUCCESS)
}

fn run_open(config: &Config) -> wlcontrol_core::WlResult<ExitCode> {
    wlcontrol_ui::open(config)?;
    Ok(ExitCode::SUCCESS)
}

fn run_action(config: &Config, action: ActionKind) -> wlcontrol_core::WlResult<ExitCode> {
    // Display-only actions are handled here without a refresh.
    match &action {
        ActionKind::OpenControlCenter => return run_open(config),
        ActionKind::CycleDisplay => {
            // Wave 2 persists display mode; the baseline is a no-op ack.
            println!("display mode toggle acknowledged");
            return Ok(ExitCode::SUCCESS);
        }
        _ => {}
    }

    let state = current_state(config);
    let client = NixlingClient::new(config);

    match plan::plan(&action, &state, config) {
        Ok(PlannedAction::Socket { intent }) => match client.dispatch(&intent) {
            Ok(outcome) => {
                println!("{}", outcome.summary);
                Ok(ExitCode::SUCCESS)
            }
            Err(err) => {
                eprintln!("nixling-wlcontrol: {err}");
                Ok(ExitCode::from(1))
            }
        },
        Ok(PlannedAction::Process { argv }) => spawn_process(argv),
        Err(reason) => {
            eprintln!("nixling-wlcontrol: action unavailable: {reason:?}");
            Ok(ExitCode::from(1))
        }
    }
}

/// Spawn an argv-only host process (terminal launch). There is no shell
/// interpretation: the first element is the program and the rest are arguments.
fn spawn_process(argv: Vec<String>) -> wlcontrol_core::WlResult<ExitCode> {
    let Some((program, args)) = argv.split_first() else {
        return Err(wlcontrol_core::WlError::Config(
            "empty terminal argv; check [terminal] config".to_owned(),
        ));
    };
    std::process::Command::new(program).args(args).spawn()?;
    Ok(ExitCode::SUCCESS)
}
