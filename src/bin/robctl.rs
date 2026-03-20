// Binary entry point for robctl
// Uses the CLI functionality from the batman_robin crate

use batman_robin::Client;
use batman_robin::cli::*;

/// Handle a `RobinError` in a CLI-friendly way by printing the error and exiting.
fn exit_on_error<T>(res: Result<T, batman_robin::RobinError>) -> T {
    match res {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    let client = Client::new();
    let matches = app::build_cli().get_matches();
    let mesh_if = matches
        .get_one::<String>("meshif")
        .map(String::as_str)
        .unwrap_or("bat0");

    let algo_name = exit_on_error(client.get_default_routing_algo().await);
    if matches.get_flag("version") {
        println!(
            "robctl version: {} [{}]",
            env!("CARGO_PKG_VERSION"),
            algo_name
        );
        return;
    }

    match matches.subcommand() {
        Some(("neighbors", _)) => {
            let entries = exit_on_error(client.neighbors(mesh_if).await);
            neighbors::print_neighbors(&entries, algo_name.as_str());
        }
        Some(("gateways", _)) => {
            let entries = exit_on_error(client.gateways(mesh_if).await);
            gateways::print_gwl(&entries, algo_name.as_str());
        }
        Some(("gw_mode", sub_m)) => {
            let mode_str = sub_m.get_one::<String>("mode").map(String::as_str);
            let param_str = sub_m.get_one::<String>("param").map(String::as_str);

            if mode_str.is_none() {
                let entries = exit_on_error(client.get_gw_mode(mesh_if).await);
                gw_mode::print_gw(&entries);
                return;
            }

            let mode = match mode_str.unwrap() {
                "off" => batman_robin::GwMode::Off,
                "client" => batman_robin::GwMode::Client,
                "server" => batman_robin::GwMode::Server,
                other => {
                    eprintln!("Invalid mode: {}", other);
                    return;
                }
            };

            let (down, up, sel_class) = if let Some(param) = param_str {
                match gw_mode::parse_gw_param(mode, param) {
                    Ok(values) => values,
                    Err(e) => {
                        eprintln!("{}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                (None, None, None)
            };

            exit_on_error(client.set_gw_mode(mode, down, up, sel_class, mesh_if).await);
        }
        Some(("originators", _)) => {
            let entries = exit_on_error(client.originators(mesh_if).await);
            originators::print_originators(&entries, algo_name.as_str());
        }
        Some(("translocal", _)) => {
            let entries = exit_on_error(client.translocal(mesh_if).await);
            translocal::print_translocal(&entries);
        }
        Some(("transglobal", _)) => {
            let entries = exit_on_error(client.transglobal(mesh_if).await);
            transglobal::print_transglobal(&entries);
        }
        Some(("interface", sub_m)) => {
            let manual = sub_m.get_flag("manual");
            let action = sub_m.get_one::<String>("action").map(String::as_str);
            let params: Vec<&str> = match sub_m.get_many::<String>("params") {
                Some(vals) => vals.map(String::as_str).collect(),
                None => Vec::new(),
            };

            if action.is_none() {
                let entries = exit_on_error(client.get_interface(mesh_if).await);
                interface::print_interfaces(&entries);
                return;
            }

            let action = action.unwrap();
            match action {
                "destroy" | "D" => {
                    if !params.is_empty() {
                        eprintln!("Error - extra parameter after '{}'", action);
                        return;
                    }
                    exit_on_error(client.destroy_interface(mesh_if).await);
                    return;
                }
                "create" | "c" => {
                    let routing_algo = match params.as_slice() {
                        [] => None,
                        ["ra", algo] => Some(*algo),
                        ["routing_algo", algo] => Some(*algo),
                        _ => {
                            eprintln!("Error - invalid parameters for create");
                            return;
                        }
                    };

                    exit_on_error(client.create_interface(mesh_if, routing_algo).await);
                    return;
                }
                "add" | "a" | "del" | "d" => {
                    if params.is_empty() {
                        eprintln!("Error - missing interface name(s) after '{}'", action);
                        return;
                    }

                    let exists = client.if_nametoindex(mesh_if).await.unwrap_or(0);
                    if !manual && exists == 0 && action.starts_with("a") {
                        exit_on_error(client.create_interface(mesh_if, None).await);
                    }

                    let pre_count = exit_on_error(client.count_interfaces(mesh_if).await);

                    for iface in &params {
                        match action {
                            "add" | "a" => {
                                exit_on_error(client.set_interface(iface, Some(mesh_if)).await);
                            }
                            "del" | "d" => {
                                exit_on_error(client.set_interface(iface, None).await);
                            }
                            _ => unreachable!(),
                        }
                    }

                    if !manual && (action == "del" || action == "d") {
                        let cnt = exit_on_error(client.count_interfaces(mesh_if).await);

                        if cnt == 0 && pre_count > 0 {
                            println!(
                                "Warning: {} has no interfaces and can be destroyed with: robctl meshif {} interface destroy",
                                mesh_if, mesh_if
                            );
                        }
                    }
                }
                _ => {}
            }
        }
        Some(("aggregation", sub_m)) => {
            let val = sub_m.get_one::<u8>("value");
            if let Some(v) = val {
                exit_on_error(client.set_aggregation(mesh_if, *v == 1).await);
            } else {
                let enabled = exit_on_error(client.get_aggregation(mesh_if).await);
                println!("{}", if enabled { "enabled" } else { "disabled" });
            }
        }
        Some(("ap_isolation", sub_m)) => {
            let val = sub_m.get_one::<u8>("value");
            if let Some(v) = val {
                exit_on_error(client.set_ap_isolation(mesh_if, *v == 1).await);
            } else {
                let enabled = exit_on_error(client.get_ap_isolation(mesh_if).await);
                println!("{}", if enabled { "enabled" } else { "disabled" });
            }
        }
        Some(("bridge_loop_avoidance", sub_m)) => {
            let val = sub_m.get_one::<u8>("value");
            if let Some(v) = val {
                exit_on_error(client.set_bridge_loop_avoidance(mesh_if, *v == 1).await);
            } else {
                let enabled = exit_on_error(client.get_bridge_loop_avoidance(mesh_if).await);
                println!("{}", if enabled { "enabled" } else { "disabled" });
            }
        }
        Some(("routing_algo", sub_m)) => {
            let param = sub_m.get_one::<String>("value");
            if let Some(algo) = param {
                exit_on_error(client.set_default_routing_algo(algo).await);
                return;
            }

            // Active routing algos
            let active = exit_on_error(client.get_active_routing_algos().await);
            if !active.is_empty() {
                println!("Active routing protocol configuration:");
                for (iface, algo) in &active {
                    println!(" * {}: {}", iface, algo);
                }
                println!();
            }

            // Default routing algo
            let default_algo = exit_on_error(client.get_default_routing_algo().await);
            println!("Selected routing algorithm (used when next batX interface is created):");
            println!(" => {}\n", default_algo);

            // Available routing algos
            let available = exit_on_error(client.get_available_routing_algos().await);
            println!("Available routing algorithms:");
            for algo in available {
                println!(" * {}", algo);
            }
        }
        _ => unreachable!("Subcommand required"),
    }
}
