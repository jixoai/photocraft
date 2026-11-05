use super::*;

#[test]
fn every_command_has_a_described_object_schema_with_explicit_coverage() {
    let mut complete = 0usize;
    let mut partial = 0usize;
    for spec in crate::command_specs() {
        let schema = input_schema(spec);
        assert_eq!(schema["$schema"], DIALECT, "{}", spec.id);
        assert_eq!(schema["type"], "object", "{}: {schema}", spec.id);
        assert!(schema["properties"].is_object(), "{}: {schema}", spec.id);
        assert!(schema["description"].as_str().is_some_and(|text| !text.is_empty()), "{}", spec.id);
        match schema["x-photocraft-coverage"].as_str() {
            Some("complete") => complete += 1,
            Some("partial") => partial += 1,
            other => panic!("{}: unexpected coverage {other:?}", spec.id),
        }
        for property in schema["properties"].as_object().unwrap().values() {
            assert!(property["description"].is_string(), "{}: {property}", spec.id);
        }
    }
    println!("command schemas: {complete} complete, {partial} partial");
    assert!(complete >= 135);
    assert_eq!(complete + partial, crate::command_specs().len());
}

#[test]
fn core_schemas_describe_defaults_ranges_requiredness_and_units() {
    let new = input_schema(crate::commands::find("file.new").unwrap());
    assert_eq!(new["properties"]["width"]["default"], 1920);
    assert_eq!(new["properties"]["height"]["maximum"], 300000);
    assert_eq!(new["properties"]["depth"]["enum"], json!([8, 16, 32]));
    assert_eq!(new["required"], json!([]));
    assert_eq!(new["additionalProperties"], false);
    assert!(new["properties"]["resolution"]["description"].as_str().unwrap().contains("pixels per inch"));
    let sample = input_schema(crate::commands::find("document.pixel").unwrap());
    assert_eq!(sample["required"], json!([]));
    assert_eq!(sample["properties"]["x"]["default"], 0);
    assert_eq!(new["properties"]["backgroundColor"]["maxItems"], 3);
    assert!(new["properties"].get("coalesce").is_none());
}

#[test]
fn case_insensitive_choice_schemas_escape_literal_punctuation() {
    let schema = types::choices(&["A+B", "space (name)", "path\\name"], "Literal case-insensitive choices.");
    jsonschema::meta::validate(&schema).unwrap();
    for value in ["a+b", "A+B", "SPACE (NAME)", "PATH\\NAME"] {
        jsonschema::validate(&schema, &json!(value)).unwrap();
    }
    for value in ["aaab", "space name", "other", "path/name", "prefix a+b"] {
        assert!(jsonschema::validate(&schema, &json!(value)).is_err(), "{value}");
    }
}

#[test]
fn legacy_notation_recovers_nested_types_without_claiming_complete_contracts() {
    let schema = notation::schema(
        r##"{"size":0.5..5000 px?,"mode":"normal|multiply|screen"="normal","mask":{"enabled":bool?,"density":0..100=100}?,"points":[[x,y,pressure?],…],"name":str?} → result"##,
    );
    assert_eq!(schema["properties"]["size"]["minimum"], 0.5);
    assert_eq!(schema["properties"]["size"]["maximum"], 5000.0);
    assert_eq!(schema["properties"]["mode"]["enum"], json!(["normal", "multiply", "screen"]));
    assert_eq!(schema["properties"]["mode"]["default"], "normal");
    assert_eq!(schema["properties"]["mask"]["properties"]["enabled"]["type"], "boolean");
    assert_eq!(schema["properties"]["points"]["items"]["minItems"], 2);
    assert_eq!(schema["properties"]["name"]["type"], "string");
    assert_eq!(schema["additionalProperties"], true);
}

#[test]
fn grouped_keys_quoted_commas_and_unicode_notes_do_not_leak_into_root_properties() {
    let schema = notation::schema(r##"{"x","y":px?,"text":str? (new name: "A, B"),"nested":{"flag":bool},…更多参数} → {"output":str}"##);
    let fields = schema["properties"].as_object().unwrap();
    assert!(fields.contains_key("x") && fields.contains_key("y"));
    assert!(!fields.contains_key("flag"));
    assert!(!fields.contains_key("output"));
}

#[test]
fn deeply_nested_or_incomplete_notation_is_bounded_and_never_panics() {
    let nested = format!("{{\"nested\":{}0{}}}", "[".repeat(256), "]".repeat(256));
    for text in ["", "not JSON", "{", "{\"bad", "{\"x\":0..=10}", &nested] {
        let schema = notation::schema(text);
        assert_eq!(schema["type"], "object", "{text}");
        assert!(schema["properties"].is_object(), "{text}");
    }
}

#[test]
fn every_schema_compiles_against_the_real_draft_2020_12_metaschema() {
    for spec in crate::command_specs() {
        let schema = crate::commands::input_schema(spec.id).unwrap();
        jsonschema::meta::validate(schema).unwrap_or_else(|error| panic!("{}: {error}", spec.id));
        let validator = jsonschema::validator_for(schema).unwrap_or_else(|error| panic!("{}: {error}", spec.id));
        for example in schema.get("examples").and_then(Value::as_array).into_iter().flatten() {
            validator.validate(example).unwrap_or_else(|error| panic!("{} example {example}: {error}", spec.id));
        }
        check_defaults(spec.id, schema);
    }
}

fn check_defaults(id: &str, schema: &Value) {
    if let Some(default) = schema.get("default") {
        jsonschema::validate(schema, default).unwrap_or_else(|error| panic!("{id} default {default}: {error}; schema={schema}"));
    }
    for keyword in ["properties", "$defs"] {
        for child in schema.get(keyword).and_then(Value::as_object).into_iter().flat_map(|fields| fields.values()) {
            check_defaults(id, child);
        }
    }
    for keyword in ["items", "additionalProperties", "if", "then", "else", "not"] {
        if let Some(child) = schema.get(keyword) {
            check_defaults(id, child);
        }
    }
    for keyword in ["anyOf", "allOf", "oneOf", "prefixItems"] {
        for child in schema.get(keyword).and_then(Value::as_array).into_iter().flatten() {
            check_defaults(id, child);
        }
    }
}

#[test]
fn defaults_and_layer_selector_variants_match_execution() {
    let mut session = crate::Session::new();
    session.execute("file.new", json!({"width":8,"height":8,"profile":"none"})).unwrap();
    for params in [json!({}), json!({"x":0,"y":0}), json!({"x":i32::MIN,"y":i32::MAX})] {
        jsonschema::validate(crate::commands::input_schema("document.pixel").unwrap(), &params).unwrap();
        session.execute("document.pixel", params).unwrap();
    }
    let layer = session.execute("layer.new.layer", json!({"name":"Detail"})).unwrap()["layer"].clone();
    for selector in [layer, json!("detail"), json!({"name":"Detail"}), json!({"background":true}), json!({"index":1}), json!({"ordinal":"Trgt"}), Value::Null] {
        for command in ["layer.hideLayers", "layer.showLayers"] {
            let params = json!({"layer":selector});
            jsonschema::validate(crate::commands::input_schema(command).unwrap(), &params).unwrap();
            session.execute(command, params).unwrap();
        }
    }
    let schema = crate::commands::input_schema("file.new").unwrap();
    for params in [json!({"backgroundColor":[0,0,0,1]}), json!({"coalesce":"new"}), json!({"width":0})] {
        assert!(jsonschema::validate(schema, &params).is_err());
        assert!(session.execute("file.new", params).is_err());
    }
}

#[test]
fn shape_examples_validate_execute_and_remain_editable() {
    let mut session = crate::Session::new();
    session.execute("file.new", json!({"width":256,"height":160})).unwrap();
    let schema = crate::commands::input_schema("shape.create").unwrap();
    for example in schema["examples"].as_array().unwrap() {
        jsonschema::validate(schema, example).unwrap();
        let result = session.execute("shape.create", example.clone()).unwrap();
        let params = json!({"layer":result["layer"],"move":[4,2],"stroke":{"width":2}});
        jsonschema::validate(crate::commands::input_schema("shape.edit").unwrap(), &params).unwrap();
        session.execute("shape.edit", params).unwrap();
        session.execute("edit.undo", json!({})).unwrap();
        session.execute("edit.redo", json!({})).unwrap();
        let info = session.execute("shape.info", json!({"layer":result["layer"]})).unwrap();
        assert!(info["path"]["subpaths"].is_array());
    }
    for invalid in [json!({}), json!({"kind":"line","from":[0,0]}), json!({"kind":"path"})] {
        assert!(jsonschema::validate(schema, &invalid).is_err(), "{invalid}");
        assert!(session.execute("shape.create", invalid).is_err());
    }
}

#[test]
fn cached_schemas_are_stable_and_discovered_even_when_disabled() {
    assert!(crate::commands::input_schema("missing.command").is_none());
    let session = crate::Session::new();
    let command = crate::commands::find("shape.create").unwrap();
    let cached = crate::commands::input_schema(command.id).unwrap();
    assert!(std::ptr::eq(cached, crate::commands::input_schema(command.id).unwrap()));
    let entry = crate::commands::list_entry(&session, command, (command.enabled)(&session).err());
    assert_eq!(entry["enabled"], false);
    assert_eq!(entry["inputSchema"], *cached);
}

#[test]
fn command_list_reuses_contracts_but_refreshes_dynamic_enablement() {
    let mut session = crate::Session::new();
    let cached = crate::commands::input_schema("shape.create").unwrap();
    let before = session.execute("command.list", json!({})).unwrap();
    let entries = before.as_array().unwrap();
    assert_eq!(entries.len(), crate::command_specs().len());
    for entry in entries {
        let id = entry["id"].as_str().unwrap();
        assert_eq!(entry["inputSchema"], *crate::commands::input_schema(id).unwrap(), "{id}");
        assert_eq!(entry["description"], entry["inputSchema"]["description"], "{id}");
    }
    let shape = entries.iter().find(|entry| entry["id"] == "shape.create").unwrap();
    assert_eq!(shape["enabled"], false);
    assert_eq!(shape["needsDocument"], true);
    session.execute("file.new", json!({"width":8,"height":8})).unwrap();
    let after = session.execute("command.list", json!({})).unwrap();
    let shape = after.as_array().unwrap().iter().find(|entry| entry["id"] == "shape.create").unwrap();
    assert_eq!(shape["enabled"], true);
    assert_eq!(shape["disabledReason"], Value::Null);
    assert_eq!(shape["inputSchema"], *cached);
}

#[test]
fn painting_and_text_examples_use_the_advertised_units_and_character_offsets() {
    let mut session = crate::Session::new();
    session.execute("file.new", json!({"width":128,"height":96})).unwrap();
    session.execute("layer.new.layer", json!({"name":"Paint"})).unwrap();
    for command in ["paint.gradient", "paint.stroke", "type.create"] {
        let schema = crate::commands::input_schema(command).unwrap();
        for example in schema["examples"].as_array().unwrap() {
            jsonschema::validate(schema, example).unwrap();
            session.execute(command, example.clone()).unwrap();
        }
    }
    let edit = json!({"text":"A🦀字B","replace":{"start":1,"end":2,"text":"é"}});
    jsonschema::validate(crate::commands::input_schema("type.edit").unwrap(), &edit).unwrap();
    let result = session.execute("type.edit", edit).unwrap();
    assert_eq!(result["text"], "Aé字B");
    let style = json!({"range":[1,3],"size":18,"tracking":25,"leading":"auto","horizontalScale":100,"features":{"liga":false},"variations":{"wght":650}});
    jsonschema::validate(crate::commands::input_schema("type.setStyle").unwrap(), &style).unwrap();
    session.execute("type.setStyle", style).unwrap();
    assert_eq!(crate::commands::input_schema("paint.gradient").unwrap()["properties"]["opacity"]["maximum"], 100);
    assert_eq!(crate::commands::input_schema("paint.stroke").unwrap()["properties"]["opacity"]["maximum"], 1);
    assert_eq!(crate::commands::input_schema("paint.stroke").unwrap()["x-photocraft-coverage"], "complete");
}

#[test]
fn strict_filter_parameter_schemas_share_execution_validation() {
    let mut checked = 0;
    for command in crate::command_specs() {
        let Some(schema) = crate::filters::input_schema(command.id) else { continue };
        for name in schema["properties"].as_object().unwrap().keys() {
            for value in [json!(null), json!(true), json!("invalid-choice"), json!(0), json!(1), json!(-1), json!(u64::MAX), json!(1.5), json!([]), json!({})] {
                let params = json!({name:value});
                assert_eq!(
                    jsonschema::validate(&schema, &params).is_ok(),
                    crate::filters::validate_params(command.id, &params).is_ok(),
                    "{}: {params}",
                    command.id
                );
            }
        }
        checked += 1;
    }
    assert!(checked >= 30);
}
