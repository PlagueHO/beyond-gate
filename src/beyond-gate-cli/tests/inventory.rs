//! Keeps the shared manual-suite inventory aligned with the actual clap parser.

use std::collections::BTreeMap;

use clap::Command;
use serde_json::Value;

/// Collects every explicit command and argument, excluding clap's generated help.
fn commands(command: &Command, path: &str, result: &mut BTreeMap<String, Vec<String>>) {
    let mut arguments: Vec<_> = command
        .get_arguments()
        .map(|argument| argument.get_id().to_string())
        .collect();
    arguments.sort();
    result.insert(path.to_owned(), arguments);
    for child in command.get_subcommands() {
        let path = format!("{path} {}", child.get_name()).trim().to_owned();
        commands(child, &path, result);
    }
}

/// Requires an inventory update for commands, formatting controls, shells, or clients.
#[test]
fn manual_inventory_matches_parser() {
    let inventory: Value =
        serde_json::from_str(include_str!("../../../scripts/e2e/capabilities.json"))
            .expect("inventory JSON");
    let command = beyond_gate_cli::command();
    let mut actual = BTreeMap::new();
    commands(&command, "", &mut actual);
    let expected: BTreeMap<String, Vec<String>> = inventory["cli"]
        .as_array()
        .expect("CLI inventory")
        .iter()
        .map(|entry| {
            (
                entry["path"].as_str().expect("path").to_owned(),
                serde_json::from_value(entry["arguments"].clone()).expect("argument names"),
            )
        })
        .collect();
    assert_eq!(actual, expected);
    for (path, argument, key) in [
        ("character list", "format", "formats"),
        ("campaign list", "format", "formats"),
        ("book list", "format", "formats"),
        ("book outline", "format", "formats"),
        ("completions", "shell", "shells"),
        ("mcp install", "client", "clients"),
    ] {
        let mut selected = &command;
        for component in path.split(' ') {
            selected = selected.find_subcommand(component).expect("command");
        }
        let argument = selected
            .get_arguments()
            .find(|value| value.get_id() == argument)
            .expect("argument");
        let mut values: Vec<_> = argument
            .get_value_parser()
            .possible_values()
            .expect("enumerated values")
            .map(|value| value.get_name().to_owned())
            .collect();
        values.sort();
        assert_eq!(
            serde_json::to_value(values).expect("values"),
            inventory[key]
        );
    }
}
