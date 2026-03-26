use crate::Gateway;
use crate::GatewayEvent;

use clap::Command;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table};
use std::time::UNIX_EPOCH;

/// Creates the CLI command for gateway operations.
///
/// # Returns
/// - A `clap::Command` configured with:
///   - Name: `"gateways"`
///   - Alias: `"gwl"`
///   - Subcommands:
///     - `list` (default): Display the list of gateways
///     - `listen`: Subscribe to gateway change events
///   - Version flag disabled
pub fn cmd_gateways() -> Command {
    Command::new("gateways")
        .alias("gwl")
        .about("Display or monitor gateways.")
        .long_about("Display the list of gateways or listen for gateway change events")
        .override_usage("\trobctl [options] gateways|gwl [COMMAND] [options]\n")
        .disable_version_flag(true)
        .subcommand(cmd_gateways_list())
        .subcommand(cmd_gateways_listen())
        // Make 'list' the default if no subcommand is specified
        .args_conflicts_with_subcommands(true)
}

/// Subcommand: gateways list (display gateway list)
fn cmd_gateways_list() -> Command {
    Command::new("list")
        .about("Display the list of gateways (default)")
        .long_about("Display all gateways known to the mesh interface")
        .override_usage("\trobctl [options] gateways list\n")
        .disable_version_flag(true)
}

/// Subcommand: gateways listen (subscribe to gateway events)
fn cmd_gateways_listen() -> Command {
    Command::new("listen")
        .about("Listen for gateway change events")
        .long_about(
            "Monitor gateway changes in real-time.\n\
             Events: ADD (gateway selected), CHANGE (better gateway found), DELETE (selected gateway lost).\n\
             Uses batman-adv kernel uevents via netlink."
        )
        .override_usage("\trobctl [options] gateways listen [options]\n")
        .disable_version_flag(true)
}

/// Prints a header for the gateway event listener output.
pub fn print_gateway_event_header(mesh_if: &str) {
    println!("Listening for gateway events on {mesh_if}. Press Ctrl+C to stop.");
    println!("{:>12}  {:<7}  GATEWAY", "UNIX_TS", "ACTION");
}

/// Prints a single gateway event.
pub fn print_gateway_event(event: &GatewayEvent) {
    let timestamp = event
        .timestamp
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    let gateway = event
        .gateway_mac
        .map(|mac| mac.to_string())
        .unwrap_or_else(|| "none".to_string());

    println!("{:>12}  {:<7}  {}", timestamp, event.action, gateway);
}

/// Prints a formatted table of gateways to the console.
///
/// # Arguments
/// - `entries`: Slice of `Gateway` entries to display.
/// - `algo_name`: Name of the BATMAN algorithm used (`"BATMAN_IV"` or `"BATMAN_V"`).
///
/// # Behavior
/// - Configures the table headers differently depending on the algorithm:
///   - `"BATMAN_IV"`: Router, TQ, Next Hop, OutgoingIF, Bandwidth Down, Bandwidth Up
///   - `"BATMAN_V"`: Router, Throughput, Next Hop, OutgoingIF, Bandwidth Down, Bandwidth Up
/// - Highlights the best gateway with an asterisk (`*`) before the MAC address.
/// - Displays optional fields (`TQ`, `Throughput`, Bandwidth) with `0` if missing.
pub fn print_gwl(entries: &[Gateway], algo_name: &str) {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    match algo_name {
        "BATMAN_IV" => {
            table.set_header(vec![
                Cell::new("Router").set_alignment(CellAlignment::Center),
                Cell::new("TQ").set_alignment(CellAlignment::Center),
                Cell::new("Next Hop").set_alignment(CellAlignment::Center),
                Cell::new("OutgoingIF").set_alignment(CellAlignment::Center),
                Cell::new("Bandwidth Down (Mbit/s)").set_alignment(CellAlignment::Center),
                Cell::new("Bandwidth Up (Mbit/s)").set_alignment(CellAlignment::Center),
            ]);
        }
        "BATMAN_V" => {
            table.set_header(vec![
                Cell::new("Router").set_alignment(CellAlignment::Center),
                Cell::new("Throughput").set_alignment(CellAlignment::Center),
                Cell::new("Next Hop").set_alignment(CellAlignment::Center),
                Cell::new("OutgoingIF").set_alignment(CellAlignment::Center),
                Cell::new("Bandwidth Down (Mbit/s)").set_alignment(CellAlignment::Center),
                Cell::new("Bandwidth Up (Mbit/s)").set_alignment(CellAlignment::Center),
            ]);
        }
        _ => return,
    }

    for g in entries {
        let router_text = if g.is_best {
            format!("* {}", g.mac_addr)
        } else {
            g.mac_addr.to_string()
        };
        let router_cell = Cell::new(router_text);
        let next_hop_cell = Cell::new(g.router.to_string());

        match algo_name {
            "BATMAN_IV" => {
                table.add_row(vec![
                    router_cell.set_alignment(CellAlignment::Right),
                    Cell::new(g.tq.unwrap_or(0)),
                    next_hop_cell,
                    Cell::new(&g.outgoing_if),
                    Cell::new(g.bandwidth_down.unwrap_or(0)),
                    Cell::new(g.bandwidth_up.unwrap_or(0)),
                ]);
            }
            "BATMAN_V" => {
                table.add_row(vec![
                    router_cell.set_alignment(CellAlignment::Right),
                    Cell::new(g.throughput.unwrap_or(0)),
                    next_hop_cell,
                    Cell::new(&g.outgoing_if),
                    Cell::new(g.bandwidth_down.unwrap_or(0)),
                    Cell::new(g.bandwidth_up.unwrap_or(0)),
                ]);
            }
            _ => {}
        }
    }

    println!("{table}");
}
