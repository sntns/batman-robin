use crate::Link;

use clap::{Arg, Command};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, CellAlignment, ContentArrangement, Table};

/// Creates the CLI command for displaying the one-hop link table.
///
/// # Returns
/// - A `clap::Command` configured with:
///   - Name: `"links"`
///   - Alias: `"lk"`
///   - Short and long description: `"Display the one-hop link-quality table."`
///   - Usage override:
///       ```text
///       robctl [options] links|lk [options]
///       ```
///   - Optional `--format`/`-f` option (`table` or `json`, default `table`)
///   - Version flag disabled
pub fn cmd_links() -> Command {
    Command::new("links")
        .alias("lk")
        .about("Display the one-hop link-quality table.")
        .long_about(
            "Display the raw one-hop link-quality data (TQ or throughput) towards each \
             directly reachable neighbor, suitable as edge data for a mesh-wide link-quality \
             graph.",
        )
        .override_usage("\trobctl [options] links|lk [options]\n")
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .value_name("FORMAT")
                .value_parser(["table", "json"])
                .default_value("table")
                .help("Output format (table|json)"),
        )
        .disable_version_flag(true)
}

/// Prints a formatted one-hop link table.
///
/// # Arguments
/// - `entries`: Slice of `Link` entries.
/// - `algo_name`: Name of the routing algorithm (BATMAN_IV or BATMAN_V).
/// - `format`: Either `"table"` for a human-readable table, or `"json"` for a
///   machine-readable JSON array (one object per link).
///
/// # Behavior
/// - For BATMAN_IV:
///     - Columns: `"Local"`, `"Neighbor"`, `"Outgoing IF"`, `"TQ"`, `"Last seen"`
///     - TQ is displayed as `value/255`
/// - For BATMAN_V:
///     - Columns: `"Local"`, `"Neighbor"`, `"Outgoing IF"`, `"Throughput (Mbit/s)"`, `"Last seen"`
/// - `last_seen_ms` is formatted as seconds with milliseconds precision (table format only).
pub fn print_links(entries: &[Link], algo_name: &str, format: &str) {
    if format == "json" {
        match serde_json::to_string_pretty(entries) {
            Ok(json) => println!("{json}"),
            Err(e) => eprintln!("Failed to serialize links to JSON: {e}"),
        }
        return;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    match algo_name {
        "BATMAN_IV" => {
            table.set_header(vec![
                Cell::new("Local").set_alignment(CellAlignment::Center),
                Cell::new("Neighbor").set_alignment(CellAlignment::Center),
                Cell::new("Outgoing IF").set_alignment(CellAlignment::Center),
                Cell::new("TQ").set_alignment(CellAlignment::Center),
                Cell::new("Last seen").set_alignment(CellAlignment::Center),
            ]);
        }
        "BATMAN_V" => {
            table.set_header(vec![
                Cell::new("Local").set_alignment(CellAlignment::Center),
                Cell::new("Neighbor").set_alignment(CellAlignment::Center),
                Cell::new("Outgoing IF").set_alignment(CellAlignment::Center),
                Cell::new("Throughput (Mbit/s)").set_alignment(CellAlignment::Center),
                Cell::new("Last seen").set_alignment(CellAlignment::Center),
            ]);
        }
        _ => return,
    }

    for l in entries {
        let last_seen_secs = l.last_seen_ms / 1000;
        let last_seen_msecs = l.last_seen_ms % 1000;
        let last_seen = format!("{}.{:03}s", last_seen_secs, last_seen_msecs);

        match algo_name {
            "BATMAN_IV" => {
                let tq = l.tq.unwrap_or(0);

                table.add_row(vec![
                    Cell::new(l.local.to_string()),
                    Cell::new(l.neighbor.to_string()),
                    Cell::new(&l.outgoing_if),
                    Cell::new(format!("{}/255", tq)),
                    Cell::new(last_seen),
                ]);
            }
            "BATMAN_V" => {
                let throughput_cell = match l.throughput_kbps {
                    Some(kbits) => {
                        let mbit = kbits / 1000;
                        let rest = (kbits % 1000) / 100;

                        Cell::new(format!("{mbit}.{rest}"))
                    }
                    None => Cell::new("-"),
                };

                table.add_row(vec![
                    Cell::new(l.local.to_string()),
                    Cell::new(l.neighbor.to_string()),
                    Cell::new(&l.outgoing_if),
                    throughput_cell,
                    Cell::new(last_seen),
                ]);
            }
            _ => {}
        }
    }

    println!("{table}");
}
