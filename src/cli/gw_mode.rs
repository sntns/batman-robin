use crate::{Error, GatewayInfo, GwMode};

use clap::{Arg, Command};

type GwParseResult = (Option<u32>, Option<u32>, Option<u32>);

/// Creates the CLI command for displaying or modifying the gateway mode.
///
/// # Returns
/// - A `clap::Command` configured with:
///   - Name: `"gw_mode"`
///   - Alias: `"gw"`
///   - Short and long description: `"Display or modify the gateway mode."`
///   - Usage override: `robctl [options] gw_mode|gw [options] [mode] [sel_class|bandwidth]`
///   - Optional positional arguments:
///     - `"mode"`: Gateway mode (`off`, `client`, or `server`)
///     - `"param"`: Gateway parameter (selection class or bandwidth)
///   - Version flag disabled
pub fn cmd_gw_mode() -> Command {
    Command::new("gw_mode")
        .alias("gw")
        .about("Display or modify the gateway mode.")
        .long_about("Display or modify the gateway mode.")
        .override_usage("\trobctl [options] gw_mode|gw [options] [mode] [sel_class|bandwidth]\n")
        .arg(
            Arg::new("mode")
                .value_name("mode")
                .required(false)
                .help("Gateway mode (off|client|server)"),
        )
        .arg(
            Arg::new("param")
                .value_name("sel_class|bandwidth")
                .required(false)
                .help("Gateway parameter (selection class or bandwidth)"),
        )
        .disable_version_flag(true)
}

/// Prints a human-readable representation of the current gateway configuration.
///
/// # Arguments
/// - `info`: `GatewayInfo` struct containing mode, algorithm, selection class, and bandwidth.
///
/// # Behavior
/// - Displays different formats depending on the `GwMode`:
///   - `Off`: prints `"off"`
///   - `Client`: prints `"client (selection class: ... MBit)"`
///   - `Server`: prints `"server (announced bw: down/up MBit)"`
///   - `Unknown`: prints `"unknown"`
/// - For BATMAN_V algorithm, selection class is printed with one decimal place.
pub fn print_gw(info: &GatewayInfo) {
    match info.mode {
        GwMode::Off => {
            println!("off");
        }
        GwMode::Client => {
            if info.algo == "BATMAN_V" {
                println!(
                    "client (selection class: {}.{} MBit)",
                    info.sel_class / 10,
                    info.sel_class % 10
                );
            } else {
                println!("client (selection class: {} MBit)", info.sel_class,);
            }
        }
        GwMode::Server => {
            let down = info.bandwidth_down;
            let up = info.bandwidth_up;

            println!(
                "server (announced bw: {}.{}/{}.{} MBit)",
                down / 10,
                down % 10,
                up / 10,
                up % 10
            );
        }
        GwMode::Unknown => {
            println!("unknown");
        }
    }
}

/// Parses a gateway parameter string according to the gateway mode.
///
/// # Arguments
/// - `mode`: The `GwMode` to interpret the parameter for.
/// - `param`: The parameter string, e.g., selection class or bandwidth (`"1000/500"`).
///
/// # Returns
/// - `Ok((Option<down>, Option<up>, Option<sel_class>))`
///   - `down` and `up` are bandwidth values in kbit for `Server` mode.
///   - `sel_class` is used for `Client` mode.
///   - `None` values for `Off` mode.
/// - `Err(Error)` if parsing fails or mode is `Unknown`.
///
/// # Notes
/// - For server mode, the `param` can be `"down/up"` and supports optional `"kbit"` or `"MBit"` suffix.
/// - For client mode, `param` is parsed as a selection class integer.
pub fn parse_gw_param(mode: GwMode, param: &str) -> Result<GwParseResult, Error> {
    match mode {
        GwMode::Off => Ok((None, None, None)),
        GwMode::Client => {
            let sel_class = param
                .parse::<u32>()
                .map_err(|e| Error::Argument(format!("Invalid sel_class '{}': {:?}", param, e)))?;
            Ok((None, None, Some(sel_class)))
        }
        GwMode::Server => {
            let parts: Vec<&str> = param.split('/').collect();
            let parse_value = |s: &str| -> Result<u32, Error> {
                let s = s.trim().to_lowercase();
                if s.ends_with("kbit") {
                    Ok(s.trim_end_matches("kbit").parse::<u32>().map_err(|e| {
                        Error::Argument(format!("Invalid bandwidth '{}': {:?}", s, e))
                    })?)
                } else if s.ends_with("mbit") {
                    let val = s.trim_end_matches("mbit").parse::<u32>().map_err(|e| {
                        Error::Argument(format!("Invalid bandwidth '{}': {:?}", s, e))
                    })?;
                    Ok(val * 1000)
                } else {
                    Ok(s.parse::<u32>().map_err(|e| {
                        Error::Argument(format!("Invalid bandwidth '{}': {:?}", s, e))
                    })?)
                }
            };

            let down = parse_value(parts[0])?;
            let up = if let Some(u) = parts.get(1) {
                Some(parse_value(u)?)
            } else {
                Some(down / 5)
            };

            Ok((Some(down), up, None))
        }
        GwMode::Unknown => Err(Error::NotFound("Unknown mode".to_string())),
    }
}
