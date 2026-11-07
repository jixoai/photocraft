use serde_json::{Value, json};

use super::{described, object, types};

pub(super) fn description(id: &str) -> Option<&'static str> {
    Some(match id {
        "shape.create" => {
            "Create an editable vector shape layer, or append components to addTo. Use a live primitive for adjustable geometry or kind=path for Bézier curves. rect is [x,y,width,height] in document pixels, not corner coordinates. fill/stroke preserve separate editable paints. Returns shape.info; do not rasterize unless pixel editing is required."
        }
        "shape.edit" => {
            "Edit an existing vector shape without flattening it. Omitted properties remain unchanged; fill=null removes the fill and stroke=null removes the outline. A new path replaces the geometry and drops live-shape parameters. kind=path requires path and uses it instead of live-primitive geometry. move/transform use document pixels. Inspect shape.info before editing a component."
        }
        "path.set" => {
            "Create or replace the work path or a named path. Does not paint pixels. op appends the new subpaths to an existing path using that Boolean operation. Use shape.create for an editable painted shape, path.fill for raster fill, or path.toSelection for a selection."
        }
        "path.delete" => "Delete the work path or a named path without changing already painted pixels. The work path is the default.",
        "path.rename" => {
            "Rename a saved path, or save the work path under a name. An existing destination name is an error; the work path cannot be the destination."
        }
        "path.clippingPath.set" => {
            "Choose a saved path as the document's clipping path for export. This is different from a layer clipping mask; flatness controls export curve approximation."
        }
        "path.clippingPath.clear" => "Remove the document's clipping-path designation without deleting the saved path.",
        _ => return None,
    })
}

pub(super) fn schema(id: &str) -> Option<Value> {
    Some(match id {
        "shape.create" => {
            let mut schema = shape(false);
            schema["allOf"] = json!([
                {"if":{"properties":{"kind":{"const":"path"}},"required":["kind"]},"then":{"required":["path"]}},
                {"if":{"properties":{"kind":{"const":"line"}},"required":["kind"]},"then":{"required":["from","to"]}},
                {"if":{"not":{"properties":{"kind":{"enum":["path","line"]}},"required":["kind"]}},"then":{"required":["rect"]}}
            ]);
            schema["examples"] = json!([
                {"kind":"ellipse","rect":[32,24,160,96],"fill":"#2f718c","stroke":{"width":3,"color":"#163b4a"},"name":"Ellipse"},
                {"kind":"path","path":{"subpaths":[{"closed":false,"knots":[{"anchor":[10,40],"out":[40,0]},{"anchor":[100,40],"in":[70,80]}]}]},"fill":null,"stroke":{"width":4,"color":"#163b4a","cap":"round"}}
            ]);
            schema
        }
        "shape.edit" => {
            let mut schema = shape(true);
            schema["allOf"] = json!([
                {"if":{"properties":{"kind":{"const":"path"}},"required":["kind"]},"then":{"required":["path"]}}
            ]);
            schema["examples"] = json!([{"layer":7,"move":[12,-8],"stroke":{"width":2}}, {"layer":7,"fill":null}]);
            schema
        }
        "path.set" => object(
            json!({
                "path":types::path(),
                "name":path_name(),
                "op":described(types::operation(), "Omit to replace the path. When supplied, append the new components to the existing path using this Boolean operation.")
            }),
            &["path"],
        ),
        "path.delete" => object(json!({"name":path_name()}), &[]),
        "path.rename" => object(
            json!({"name":path_name(),"to":{"type":"string","minLength":1,"description":"New saved-path name; cannot be empty, work (case-insensitive) or Work Path, and must not already exist."}}),
            &["to"],
        ),
        "path.clippingPath.set" => object(
            json!({"name":{"type":"string","minLength":1,"description":"Existing saved-path name, not the work path."},"flatness":{"type":"number","minimum":0,"maximum":100,"default":0,"description":"Export curve flatness; 0 uses automatic approximation, otherwise the export flatness scale."}}),
            &["name"],
        ),
        "path.clippingPath.clear" => object(json!({}), &[]),
        _ => return None,
    })
}

fn path_name() -> Value {
    json!({"type":"string","default":"work","description":"Saved path name, or work / Work Path / empty string for the work path. Omit for work. Path creation does not interpret layer as a layer selector."})
}

fn shape(edit: bool) -> Value {
    let mut fields = json!({
        "kind":{"type":"string","enum":["rect","rectangle","roundedRect","roundRect","ellipse","polygon","star","line","path"],"description":"Live primitive or explicit Bézier path. Creation defaults to rect; edits retain the current live kind unless supplied."},
        "rect":types::tuple(4,"[x,y,width,height] in document pixels. Required when creating a rectangle, rounded rectangle, ellipse, polygon or star; width/height are extents, not the opposite corner."),
        "radii":{"anyOf":[{"type":"number"},types::tuple(4,"Corner radii [topLeft,topRight,bottomRight,bottomLeft] in pixels.")],"description":"Rounded-corner radii: one pixel radius for all corners or four radii. A new roundedRect defaults to 10 px; a plain rect defaults to 0."},
        "radius":{"type":"number","description":"Single-radius alias of radii; use radii when providing four corner values."},
        "sides":{"type":"number","minimum":3,"maximum":100,"description":"Polygon/star vertex count, rounded to an integer; new shapes default to 5. Triangle = polygon with sides=3."},
        "starRatio":{"type":"number","minimum":0.01,"maximum":1,"description":"Inner/outer radius ratio. New star defaults to 0.5; polygon defaults to 1. Values are clamped to 0.01–1."},
        "cornerRadius":{"type":"number","minimum":0,"maximum":1000000,"description":"Rounds each polygon/star vertex by this many document pixels; new shapes default to 0."},
        "from":types::tuple(2,"Line start [x,y] in document pixels; required for a new line."),
        "to":types::tuple(2,"Line end [x,y] in document pixels; required for a new line."),
        "weight":{"type":"number","minimum":0,"description":"Line geometry thickness in pixels, not stroke width; new lines default to 1."},
        "arrowStart":{"type":"boolean","description":"Arrowhead at the line start; new lines default to false."},
        "arrowEnd":{"type":"boolean","description":"Arrowhead at the line end; new lines default to false."},
        "arrowWidth":{"type":"number","minimum":1,"maximum":4096,"description":"Arrowhead width as a percentage of line weight; default 52 for a new arrow."},
        "arrowLength":{"type":"number","minimum":1,"maximum":4096,"description":"Arrowhead length as a percentage of line weight; default 50 for a new arrow."},
        "arrowConcavity":{"type":"number","minimum":0,"maximum":100,"description":"Arrowhead concavity percentage; default 0 for a new arrow."},
        "path":types::path(),
        "fill":fill(true),
        "stroke":stroke(),
        "name":{"type":"string","description":"Layer name; creation generates a name when omitted, editing preserves it."}
    });
    if let Some(properties) = fields.as_object_mut() {
        if edit {
            properties.extend([
                ("layer".into(), types::layer()),
                ("move".into(), types::tuple(2,"Translate by [dx,dy] document pixels after transform.")),
                ("transform".into(), types::tuple(6,"Affine [a,b,c,d,e,f]: x'=a*x+c*y+e, y'=b*x+d*y+f. Applied before move.")),
                ("op".into(), types::operation()),
                ("subpath".into(), json!({"type":"integer","minimum":0,"description":"Zero-based subpath whose Boolean operation changes; omitted with op changes all but the first subpath."})),
                ("fillRule".into(), json!({"type":"string","enum":["nonzero","evenodd"],"description":"Path fill rule; omitted preserves the current rule."})),
            ]);
        } else {
            properties.insert("addTo".into(), json!({"type":"integer","minimum":0,"description":"Existing shape layer's stable numeric ID. Append geometry to it instead of creating a layer; its existing fill/stroke remain unchanged."}));
            properties.insert("op".into(), types::operation());
        }
    }
    object(fields, &[])
}

pub(super) fn fill(nullable: bool) -> Value {
    let gradient = object(
        json!({
            "stops":{"type":"array","maxItems":1024,"items":{"type":"array","prefixItems":[{"type":"number","minimum":0,"maximum":1},types::color()],"minItems":2,"maxItems":2},"description":"[position,colour] pairs; position is 0–1. Omitted defaults to black-to-white."},
            "angle":{"type":"number","description":"Gradient angle in degrees; default 90."},
            "scale":{"type":"number","description":"Gradient scale percentage; default 100, not 1."},
            "style":{"enum":["linear","radial","angle","reflected","diamond"],"default":"linear"},
            "reverse":{"type":"boolean","default":false},
            "opacityStops":{"type":"array","maxItems":1024,"items":types::tuple(2,"[position,opacity], both 0–1 (not percentage).")},
            "midpoints":{"type":"array","maxItems":1024,"items":{"type":"number","minimum":0,"maximum":1}},
            "offset":types::tuple(2,"[x,y] gradient offsets, each clamped to -1000..1000."),
            "dither":{"type":"boolean"},"align":{"type":"boolean"}
        }),
        &[],
    );
    let mut variants = vec![
        types::color(),
        object(json!({"gradient":gradient}), &["gradient"]),
        object(json!({"pattern":{"type":"string"},"scale":{"type":"number","default":100,"description":"Pattern scale percentage."}}), &["pattern"]),
    ];
    if nullable {
        variants.push(json!({"type":"null"}));
    }
    described(
        json!({"anyOf":variants}),
        if nullable {
            "Shape paint: RGB hex/array or stored Color, {gradient:{...}}, {pattern:name,scale:percent?}, or null for no fill. Creation defaults to the foreground; editing preserves omitted fill. This is a whole-paint replacement, not a partial gradient patch."
        } else {
            "Stroke paint: RGB hex/array or stored Color, {gradient:{...}}, or {pattern:name,scale:percent?}. Null is not a stroke paint; set the whole stroke to null to remove it."
        },
    )
}

fn stroke() -> Value {
    described(
        json!({"anyOf":[{"type":"null"},{"const":false},object(json!({
            "width":{"type":"number","minimum":0,"maximum":1000000,"description":"Outline width in document pixels."},
            "color":fill(false),"fill":fill(false),
            "opacity":{"type":"number","minimum":0,"maximum":100,"description":"Outline opacity in percent (0–100), unlike layer opacity (0–1)."},
            "align":{"enum":["inside","center","centre","outside"]},
            "cap":{"enum":["butt","round","square"]},
            "join":{"enum":["miter","round","bevel"]},
            "miterLimit":{"type":"number","minimum":1,"maximum":500},
            "dashes":{"type":["array","null"],"items":{"type":"number","minimum":0,"maximum":10000},"maxItems":32,"description":"Dash/gap lengths as multiples of stroke width; null/empty clears dashing."},
            "dashOffset":{"type":"number","minimum":-1000000,"maximum":1000000,"description":"Dash phase in multiples of stroke width."}
        }),&[])]}),
        "Outline configuration; null removes it. Creating without stroke means no outline. On edit, only supplied stroke properties change; color wins over the fill alias when both are supplied.",
    )
}
