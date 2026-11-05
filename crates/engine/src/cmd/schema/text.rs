use serde_json::{Value, json};

use super::{described, object, types};

pub(super) fn description(id: &str) -> Option<&'static str> {
    Some(match id {
        "type.create" => {
            "Create and activate an editable text layer. Point text uses a document-pixel baseline anchor (x,y); box=[x,y,width,height] creates paragraph text. Font size/leading/indents are points, converted through document resolution, not pixels. Omitted style keys use current type defaults and foreground colour. Query type.fonts before choosing fonts; missing-font warnings are returned."
        }
        "type.edit" => {
            "Edit text content or geometry on one explicit/active type layer. Use replace for a localized character span and runs for character styles; top-level kerning applies to range. Offsets count Unicode scalar characters, not bytes or UTF-16 code units. Existing styles are retained where possible. Geometry changes and rename share the edit's undo step; use type.setStyle for general formatting."
        }
        "type.setStyle" => {
            "Format the character range of one type layer, explicit layers, or selected type layers in one undo step. Omitted style properties remain unchanged. Offsets are Unicode scalar-character indices, sizes/leading/indents are points, scales are percentages and tracking/kerning are 1/1000 em. Explicit layers formats whole layers; a layer/range keeps single-layer scope."
        }
        _ => return None,
    })
}

pub(super) fn schema(id: &str) -> Option<Value> {
    let fields = match id {
        "type.create" => {
            let mut fields = style();
            fields.extend([
                ("x".into(), json!({"type":"number","default":0,"description":"Point-text baseline anchor x in document pixels; ignored when box is supplied."})),
                ("y".into(), json!({"type":"number","default":0,"description":"Point-text baseline anchor y in document pixels, not the glyph top; ignored when box is supplied."})),
                ("text".into(), json!({"type":"string","default":"","description":"Text content, with CRLF/CR normalized to LF. Omit to create an empty editable type layer."})),
                ("box".into(), box_rect(true)),
                ("name".into(), json!({"type":"string","description":"Layer name; omit to derive it from the text."})),
                ("orientation".into(), json!({"enum":["horizontal","vertical"],"default":"horizontal","description":"Text writing orientation."})),
            ]);
            fields
        }
        "type.setStyle" => {
            let mut fields = style();
            fields.extend([
                ("layer".into(), types::layer()),
                ("layers".into(), json!({"type":"array","items":{"type":"integer","minimum":0,"maximum":u64::MAX},"description":"Explicit numeric IDs; formats whole type layers and ignores non-type layers. An explicit layer takes precedence."})),
                ("range".into(), range()),
                ("metricsAsShown".into(), json!({"type":"boolean","default":false,"description":"Interpret size, leading and leadingBy after each layer's transform scale, as shown in the UI; otherwise use text-space points."})),
                ("leadingBy".into(), json!({"type":"number","minimum":-5000,"maximum":5000,"description":"Relative leading step in points, preserving differences between runs. Auto leading starts from its laid-out value; final leading clamps to 0.01–5000 pt."})),
                ("baselineShiftBy".into(), json!({"type":"number","minimum":-10000,"maximum":10000,"description":"Relative baseline shift in points; final shift clamps to ±5000 pt."})),
                ("trackingBy".into(), json!({"type":"number","minimum":-11000,"maximum":11000,"description":"Relative tracking step in 1/1000 em; final tracking clamps to -1000..10000."})),
            ]);
            fields
        }
        "type.edit" => {
            let mut run = character_style();
            run.insert("start".into(), json!({"type":"integer","minimum":0,"default":0,"description":"Inclusive start in Unicode scalar characters."}));
            run.insert(
                "end".into(),
                json!({"type":"integer","minimum":0,"description":"Exclusive end in Unicode scalar characters; omit for text end. Must not precede start."}),
            );
            let fields = json!({
                "layer":types::layer(),
                "text":{"type":"string","description":"Replace the whole text while preserving styles around the changed span."},
                "replace":described(object(json!({
                    "start":{"type":"integer","minimum":0,"default":0},
                    "end":{"type":"integer","minimum":0},
                    "text":{"type":"string","default":""}
                }), &[]), "Localized replacement [start,end) in Unicode scalar characters, applied after text. Omitted end means text end; omitted text deletes the span."),
                "runs":{"type":"array","items":object(Value::Object(run), &[]),"description":"Character-style patches over [start,end) spans after content edits. Not a replacement array of stored run lengths."},
                "box":box_rect(false),
                "point":types::tuple(2,"Convert to point text at document-pixel baseline anchor [x,y]. Applied after box when both are supplied."),
                "move":types::tuple(2,"Move text by [dx,dy] document pixels after box/point."),
                "transform":types::tuple(6,"Replace the full affine [a,b,c,d,e,f] transform after box/point/move: x'=a*x+c*y+e, y'=b*x+d*y+f."),
                "antialias":{"enum":["none","sharp","crisp","strong","smooth"],"description":"Text rasterization edge mode; omitted preserves the current mode."},
                "name":{"type":"string","description":"Rename the type layer in the same history step."},
                "kerning":kerning(),"range":range(),
                "kernPair":described(object(json!({
                    "at":{"type":"integer","minimum":1,"description":"Caret index between two characters (not a line break)."},
                    "by":{"type":"number","minimum":-10000,"maximum":10000,"description":"Adjust the pair's current kerning by this many 1/1000 em."}
                }), &["at","by"]), "Adjust one kerning pair relative to its current value. The caret must identify an existing pair.")
            });
            fields.as_object().cloned().unwrap_or_default()
        }
        _ => return None,
    };
    let mut schema = object(Value::Object(fields), &[]);
    if id == "type.create" {
        schema["examples"] = json!([{"text":"PhotoCraft","x":12,"y":40,"size":18,"color":"#2f718c"}]);
    }
    Some(schema)
}

fn style() -> serde_json::Map<String, Value> {
    let mut fields = character_style();
    fields.insert("align".into(), json!({"enum":["left","center","right","justify","justifyLeft","justifyCenter","justifyRight","justifyAll"],"description":"Paragraph alignment; justify and justifyLeft are aliases."}));
    for (key, description) in [
        ("firstLineIndent", "First-line indent in points."),
        ("startIndent", "Paragraph start indent in points."),
        ("endIndent", "Paragraph end indent in points."),
        ("spaceBefore", "Space before a paragraph in points."),
        ("spaceAfter", "Space after a paragraph in points."),
        ("autoLeading", "Automatic leading percentage of font size; 120 means 120%, not a 1.2 fraction."),
    ] {
        fields.insert(key.into(), json!({"type":"number","description":description}));
    }
    fields.insert("direction".into(), json!({"enum":["auto","ltr","rtl"],"description":"Paragraph bidirectional base direction."}));
    fields.insert("hyphenate".into(), json!({"type":"boolean","description":"Enable paragraph hyphenation."}));
    fields
}

fn character_style() -> serde_json::Map<String, Value> {
    let mut fields = json!({
        "font":{"type":"string","description":"Installed family name or exact PostScript face name; inspect type.fonts. Takes precedence over family."},
        "family":{"type":"string","description":"Alias of font."},
        "fontStyle":{"type":"string","description":"Face style such as Regular/Bold/Italic; resolves the installed face and its weight/slant."},
        "postscriptName":{"type":"string","description":"Explicit PostScript face identity; prefer an installed value returned by type.fonts."},
        "weight":{"type":"number","minimum":1,"maximum":1000,"description":"Font weight; clamped to 1–1000 (400 regular, 700 bold)."},
        "italic":{"type":"boolean","description":"Choose an italic font face; distinct from fauxItalic."},
        "size":{"type":"number","minimum":crate::type_cmds::TYPE_SIZE_MIN_PT,"maximum":crate::type_cmds::TYPE_SIZE_MAX_PT,"description":"Font size in points, not pixels. Omitted creation uses saved/default type style; omitted edits preserve current values."},
        "color":types::rgb(),
        "tracking":{"type":"number","minimum":crate::type_cmds::TRACKING_MIN,"maximum":crate::type_cmds::TRACKING_MAX,"description":"Tracking in 1/1000 em, not pixels."},
        "leading":{"anyOf":[{"type":"number","minimum":crate::type_cmds::LEADING_MIN_PT,"maximum":crate::type_cmds::LEADING_MAX_PT},{"const":"auto"},{"type":"null"}],"description":"Line leading in points; auto/null uses paragraph automatic leading."},
        "baselineShift":{"type":"number","minimum":-crate::type_cmds::BASELINE_SHIFT_MAX_PT,"maximum":crate::type_cmds::BASELINE_SHIFT_MAX_PT,"description":"Baseline displacement in points."},
        "horizontalScale":{"type":"number","minimum":0,"maximum":crate::type_cmds::SCALE_MAX_PERCENT,"description":"Horizontal glyph scale percentage; 100 means unchanged. Effective minimum is 1%."},
        "verticalScale":{"type":"number","minimum":0,"maximum":crate::type_cmds::SCALE_MAX_PERCENT,"description":"Vertical glyph scale percentage; 100 means unchanged. Effective minimum is 1%."},
        "kerning":kerning(),
        "caps":{"enum":["normal","small","smallCaps","all","allCaps"],"description":"Capitalization styling without changing the stored text."},
        "baseline":{"enum":["normal","superscript","subscript",null],"description":"OpenType baseline styling; null leaves the existing value unchanged."},
        "features":{"type":"object","additionalProperties":{"anyOf":[{"type":"integer","minimum":0,"maximum":65535},{"type":"boolean"}]},"description":"OpenType feature tags to values, e.g. {ss01:1,liga:false}; supplied tags merge with existing feature settings."},
        "variations":{"type":"object","additionalProperties":{"type":"number"},"description":"Variable-font axis tags to native values, e.g. {wght:650}; supplied axes merge with existing settings."},
        "language":{"type":["string","null"],"description":"Text shaping language tag; null clears the override."}
    }).as_object().cloned().unwrap_or_default();
    for (key, description) in [
        ("underline", "Underline glyphs."),
        ("strikethrough", "Strike through glyphs."),
        ("fauxBold", "Synthesize bold weight without changing the font face."),
        ("fauxItalic", "Synthesize italic slant without changing the font face."),
        ("ligatures", "Enable standard ligatures."),
        ("discretionaryLigatures", "Enable discretionary ligatures."),
    ] {
        fields.insert(key.into(), json!({"type":"boolean","description":description}));
    }
    fields
}

fn kerning() -> Value {
    described(
        json!({"anyOf":[
            {"type":"number","minimum":crate::type_cmds::KERN_MIN,"maximum":crate::type_cmds::KERN_MAX},
            {"type":"string","enum":["metrics","optical","off","none","0"]}
        ]}),
        "Manual kerning in 1/1000 em (disables automatic pair kerning), or metrics/optical/off. none and the string 0 alias off; names are case-insensitive.",
    )
}

fn range() -> Value {
    described(
        json!({"type":"array","items":{"type":"integer","minimum":0},"minItems":2,"maxItems":2}),
        "[start,end) in Unicode scalar characters, not UTF-8 bytes/UTF-16 units. Omit for the entire text; endpoints clamp to text length.",
    )
}

fn box_rect(create: bool) -> Value {
    let mut schema =
        types::tuple(4, "Paragraph text box [x,y,width,height] in document pixels, not corner coordinates. x/y locate the box top-left, not the baseline.");
    if create && let Some(fields) = schema.as_object_mut() {
        fields.remove("items");
        fields.insert(
            "prefixItems".into(),
            json!([{"type":"number"},{"type":"number"},{"type":"number","exclusiveMinimum":0},{"type":"number","exclusiveMinimum":0}]),
        );
    }
    schema
}
