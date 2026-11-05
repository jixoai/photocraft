use super::*;

fn entry(command: &str, coverage: &str) -> Value {
    json!({
        "id": command,
        "description": "A synthetic command",
        "journal": true,
        "enabled": true,
        "inputSchema": {"$schema": DIALECT, "type": "object", "properties": {}, "x-photocraft-coverage": coverage}
    })
}

#[test]
fn added_partial_commands_do_not_lower_a_percentage_floor() {
    let baseline = inventory_contracts(json!([entry("test.kept", "complete")])).unwrap();
    let current = inventory_contracts(json!([entry("test.kept", "complete"), entry("test.new", "partial")])).unwrap();
    let report = compare(&baseline, &current);
    assert!(report.passed());
    assert_eq!(report.added_commands, ["test.new"]);
    assert_eq!((report.complete, report.partial), (1, 1));
}

#[test]
fn losing_an_audited_command_cannot_be_hidden_by_a_replacement() {
    let baseline = inventory_contracts(json!([entry("test.kept", "complete")])).unwrap();
    let current = inventory_contracts(json!([entry("test.replacement", "complete")])).unwrap();
    let report = compare(&baseline, &current);
    assert!(!report.passed());
    assert_eq!(report.missing_commands, ["test.kept"]);
}

#[test]
fn removing_a_partial_command_also_requires_review() {
    let baseline = inventory_contracts(json!([entry("test.kept", "complete"), entry("test.old", "partial")])).unwrap();
    let current = inventory_contracts(json!([entry("test.kept", "complete")])).unwrap();
    assert_eq!(compare(&baseline, &current).missing_commands, ["test.old"]);
}

#[test]
fn downgrades_fail_and_promotions_are_reported() {
    let complete = inventory_contracts(json!([entry("test.command", "complete")])).unwrap();
    let partial = inventory_contracts(json!([entry("test.command", "partial")])).unwrap();
    assert_eq!(compare(&complete, &partial).downgraded_commands, ["test.command"]);
    let promotion = compare(&partial, &complete);
    assert!(promotion.passed());
    assert_eq!(promotion.promoted_commands, ["test.command"]);
}

#[test]
fn nested_schema_and_semantic_changes_require_review() {
    let original = entry("test.command", "complete");
    let baseline = inventory_contracts(json!([original.clone()])).unwrap();
    for (field, value) in
        [("$defs", json!({"Nested": {"type": "number"}})), ("properties", json!({"size": {"type": "number", "minimum": 1}})), ("required", json!(["size"]))]
    {
        let mut changed = original.clone();
        changed["inputSchema"][field] = value;
        let current = inventory_contracts(json!([changed])).unwrap();
        assert_eq!(compare(&baseline, &current).changed_contracts, ["test.command"]);
    }
    let mut description = entry("test.command", "complete");
    description["description"] = json!("Changed omission semantics");
    assert!(!compare(&baseline, &inventory_contracts(json!([description])).unwrap()).passed());
    let mut journal = entry("test.command", "complete");
    journal["journal"] = json!(false);
    assert!(!compare(&baseline, &inventory_contracts(json!([journal])).unwrap()).passed());
}

#[test]
fn live_enablement_does_not_change_the_contract_hash() {
    let baseline = inventory_contracts(json!([entry("test.command", "complete")])).unwrap();
    let mut changed = entry("test.command", "complete");
    changed["enabled"] = json!(false);
    changed["disabledReason"] = json!("No active document");
    let current = inventory_contracts(json!([changed])).unwrap();
    assert!(compare(&baseline, &current).passed());
}

#[test]
fn canonical_objects_are_order_independent_but_arrays_are_not() {
    let first: Value = serde_json::from_str(r#"{"b":{"y":2,"x":1},"a":[1,2]}"#).unwrap();
    let second: Value = serde_json::from_str(r#"{"a":[1,2],"b":{"x":1,"y":2}}"#).unwrap();
    assert_eq!(canonical(&first), canonical(&second));
    assert_ne!(canonical(&first), canonical(&json!({"b": {"y": 2, "x": 1}, "a": [2, 1]})));
}

#[test]
fn malformed_inventories_fail_without_panicking() {
    let mut missing_schema = entry("test.command", "complete");
    missing_schema["inputSchema"] = Value::Null;
    let mut missing_description = entry("test.command", "complete");
    missing_description["description"] = json!("");
    for invalid in [
        Value::Null,
        json!([]),
        json!([null]),
        json!([entry("test.command", "complete"), entry("test.command", "partial")]),
        json!([entry("test.command", "unknown")]),
        json!([missing_schema]),
        json!([missing_description]),
    ] {
        assert!(inventory_contracts(invalid).is_err());
    }
}

#[test]
fn corrupt_and_future_baselines_are_rejected() {
    let mut baseline = inventory_contracts(json!([entry("test.command", "complete")])).unwrap();
    assert!(validate_baseline(&baseline).is_ok());
    baseline.format_version = 2;
    assert!(validate_baseline(&baseline).is_err());
    baseline.format_version = 1;
    baseline.commands.get_mut("test.command").unwrap().schema_sha256 = None;
    assert!(validate_baseline(&baseline).is_err());
}

#[test]
fn cli_arguments_cannot_silently_replace_a_baseline() {
    assert!(run(&[]).is_err());
    assert!(run(&["baseline", "input", "output", "--force"]).is_err());
    assert!(run(&["check", "missing-inventory.json", "missing-baseline.json"]).is_err());
}

#[test]
fn baseline_files_require_explicit_replacement_and_reports_round_trip() {
    let directory = std::env::temp_dir().join(format!("photocraft-downstream-files-{}", std::process::id()));
    fs::create_dir(&directory).unwrap();
    let inventory = directory.join("inventory.json");
    let baseline = directory.join("baseline.json");
    let report = directory.join("report.json");
    write_json(&inventory, &json!([entry("test.command", "complete")])).unwrap();
    let inventory_path = inventory.to_str().unwrap();
    let baseline_path = baseline.to_str().unwrap();
    let report_path = report.to_str().unwrap();
    run(&["baseline", inventory_path, baseline_path]).unwrap();
    let original = fs::read(&baseline).unwrap();
    assert!(run(&["baseline", inventory_path, baseline_path]).is_err());
    assert_eq!(fs::read(&baseline).unwrap(), original);
    run(&["check", inventory_path, baseline_path, report_path]).unwrap();
    assert_eq!(read_json(&report).unwrap()["total"], 1);
    let mut changed = entry("test.command", "complete");
    changed["description"] = json!("Reviewed semantic change");
    write_json(&inventory, &json!([changed])).unwrap();
    assert!(run(&["check", inventory_path, baseline_path, report_path]).is_err());
    assert_eq!(read_json(&report).unwrap()["changedContracts"], json!(["test.command"]));
    assert_eq!(fs::read(&baseline).unwrap(), original);
    run(&["baseline", inventory_path, baseline_path, "--replace"]).unwrap();
    run(&["check", inventory_path, baseline_path]).unwrap();
    for file in [inventory, baseline, report] {
        fs::remove_file(file).unwrap();
    }
    fs::remove_dir(directory).unwrap();
}
