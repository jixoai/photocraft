use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAX_JSON_BYTES: u64 = 64 * 1024 * 1024;
const MAX_COMMANDS: usize = 100_000;
const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";
const USAGE: &str = "usage: cargo xtask downstream check INVENTORY BASELINE [REPORT]\n       cargo xtask downstream baseline INVENTORY BASELINE [--replace]";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Baseline {
    format_version: u32,
    commands: BTreeMap<String, Contract>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Contract {
    coverage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    schema_sha256: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    total: usize,
    complete: usize,
    partial: usize,
    added_commands: Vec<String>,
    promoted_commands: Vec<String>,
    missing_commands: Vec<String>,
    downgraded_commands: Vec<String>,
    changed_contracts: Vec<String>,
}

impl Report {
    fn passed(&self) -> bool {
        self.missing_commands.is_empty() && self.downgraded_commands.is_empty() && self.changed_contracts.is_empty()
    }
}

pub fn run(args: &[&str]) -> Result<(), String> {
    match args {
        ["check", inventory, baseline] | ["check", inventory, baseline, _] => {
            let current = inventory_contracts(read_json(Path::new(inventory))?)?;
            let saved: Baseline = serde_json::from_value(read_json(Path::new(baseline))?).map_err(|error| format!("baseline: {error}"))?;
            validate_baseline(&saved)?;
            let report = compare(&saved, &current);
            let encoded = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?;
            println!("{encoded}");
            if let Some(path) = args.get(3) {
                write_json(Path::new(path), &report)?;
            }
            if report.passed() {
                Ok(())
            } else {
                Err("downstream contracts changed; review the report and execution tests before explicitly replacing the baseline".into())
            }
        }
        ["baseline", inventory, baseline] | ["baseline", inventory, baseline, "--replace"] => {
            let path = Path::new(baseline);
            if path.exists() && args.len() == 3 {
                return Err("baseline already exists; review changes and use --replace explicitly".into());
            }
            let contracts = inventory_contracts(read_json(Path::new(inventory))?)?;
            write_json(path, &contracts)?;
            println!("recorded {} commands in {}; this is a review checkpoint, not execution validation", contracts.commands.len(), path.display());
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(MAX_JSON_BYTES + 1).read_to_end(&mut bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    if bytes.len() as u64 > MAX_JSON_BYTES {
        return Err(format!("{}: JSON exceeds 64 MiB", path.display()));
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let mut encoded = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    encoded.push(b'\n');
    fs::write(path, encoded).map_err(|error| format!("{}: {error}", path.display()))
}

fn canonical(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonical).collect()),
        Value::Object(object) => {
            let ordered: BTreeMap<_, _> = object.iter().map(|(key, child)| (key.clone(), canonical(child))).collect();
            Value::Object(ordered.into_iter().collect())
        }
        _ => value.clone(),
    }
}

fn inventory_contracts(inventory: Value) -> Result<Baseline, String> {
    let entries = inventory.as_array().ok_or("inventory must be the array emitted by `photocraft-cli commands --json`")?;
    if entries.is_empty() || entries.len() > MAX_COMMANDS {
        return Err("inventory must contain between 1 and 100000 commands".into());
    }
    let mut commands = BTreeMap::new();
    for entry in entries {
        let command = entry.get("id").and_then(Value::as_str).filter(|id| !id.trim().is_empty()).ok_or("command has no nonempty id")?;
        let schema = &entry["inputSchema"];
        if schema["$schema"] != DIALECT || schema["type"] != "object" || !schema["properties"].is_object() {
            return Err(format!("{command}: missing draft 2020-12 object schema and properties"));
        }
        if !entry["description"].as_str().is_some_and(|description| !description.trim().is_empty()) || !entry["journal"].is_boolean() {
            return Err(format!("{command}: missing semantic description or journal flag"));
        }
        let coverage = schema["x-photocraft-coverage"].as_str().ok_or_else(|| format!("{command}: missing explicit coverage"))?;
        if !matches!(coverage, "complete" | "partial") {
            return Err(format!("{command}: unknown coverage {coverage}"));
        }
        let schema_sha256 = if coverage == "complete" {
            let contract = canonical(&json!({"inputSchema": schema, "description": entry["description"], "journal": entry["journal"]}));
            let encoded = serde_json::to_vec(&contract).map_err(|error| format!("{command}: {error}"))?;
            Some(crate::sha256::hex(&encoded))
        } else {
            None
        };
        if commands.insert(command.into(), Contract { coverage: coverage.into(), schema_sha256 }).is_some() {
            return Err(format!("duplicate command {command}"));
        }
    }
    Ok(Baseline { format_version: 1, commands })
}

fn validate_baseline(baseline: &Baseline) -> Result<(), String> {
    if baseline.format_version != 1 || baseline.commands.is_empty() || baseline.commands.len() > MAX_COMMANDS {
        return Err("unsupported or empty contract baseline".into());
    }
    for (command, contract) in &baseline.commands {
        let valid_hash = contract.schema_sha256.as_deref().is_some_and(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
        if command.trim().is_empty()
            || !matches!(contract.coverage.as_str(), "complete" | "partial")
            || (contract.coverage == "complete" && !valid_hash)
            || (contract.coverage == "partial" && contract.schema_sha256.is_some())
        {
            return Err(format!("{command}: invalid baseline entry"));
        }
    }
    Ok(())
}

fn compare(baseline: &Baseline, current: &Baseline) -> Report {
    let complete = current.commands.values().filter(|contract| contract.coverage == "complete").count();
    let mut report = Report {
        total: current.commands.len(),
        complete,
        partial: current.commands.len().saturating_sub(complete),
        added_commands: current.commands.keys().filter(|command| !baseline.commands.contains_key(*command)).cloned().collect(),
        promoted_commands: Vec::new(),
        missing_commands: Vec::new(),
        downgraded_commands: Vec::new(),
        changed_contracts: Vec::new(),
    };
    for (command, saved) in &baseline.commands {
        match current.commands.get(command) {
            None => report.missing_commands.push(command.clone()),
            Some(found) if saved.coverage == "complete" && found.coverage != "complete" => report.downgraded_commands.push(command.clone()),
            Some(found) if saved.coverage == "complete" && saved.schema_sha256 != found.schema_sha256 => report.changed_contracts.push(command.clone()),
            Some(found) if saved.coverage == "partial" && found.coverage == "complete" => report.promoted_commands.push(command.clone()),
            Some(_) => {}
        }
    }
    report
}

#[cfg(test)]
mod tests;
