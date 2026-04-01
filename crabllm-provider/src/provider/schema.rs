use serde_json::{Map, Value};

const MAX_DEPTH: u32 = 32;

/// Resolve all `$ref` pointers from `$defs` and remove the `$defs` key.
///
/// Walks the schema tree, replacing `{"$ref": "#/$defs/Name"}` with the
/// corresponding definition body. Handles transitive references within
/// definitions. Depth-capped at 32 to guard against pathological input.
pub fn inline_refs(schema: &mut Value) {
    let Some(obj) = schema.as_object_mut() else {
        return;
    };
    let Some(Value::Object(defs)) = obj.remove("$defs") else {
        return;
    };
    resolve(schema, &defs, 0);
}

fn resolve(node: &mut Value, defs: &Map<String, Value>, depth: u32) {
    if depth > MAX_DEPTH {
        return;
    }
    match node {
        Value::Object(map) => {
            if let Some(Value::String(r)) = map.get("$ref")
                && let Some(name) = r.strip_prefix("#/$defs/")
                && let Some(def) = defs.get(name)
            {
                *node = def.clone();
                resolve(node, defs, depth + 1);
                return;
            }
            for v in map.values_mut() {
                resolve(v, defs, depth);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                resolve(v, defs, depth);
            }
        }
        _ => {}
    }
}

/// Remove `$schema` and `$id` meta-annotations from the top level.
pub fn strip_schema_meta(schema: &mut Value) {
    if let Some(obj) = schema.as_object_mut() {
        obj.remove("$schema");
        obj.remove("$id");
    }
}

/// Flatten nullable `anyOf`/`oneOf` patterns and array type syntax.
///
/// Converts `{"anyOf": [T, {"type": "null"}]}` to `{...T, "nullable": true}`.
/// Converts `{"type": ["string", "null"]}` to `{"type": "string", "nullable": true}`.
pub fn flatten_nullable(schema: &mut Value) {
    match schema {
        Value::Object(map) => {
            // anyOf/oneOf: [T, {type: null}] → {...T, nullable: true}
            for key in ["anyOf", "oneOf"] {
                if let Some(Value::Array(variants)) = map.get(key)
                    && variants.len() == 2
                {
                    let null_idx = variants
                        .iter()
                        .position(|v| v.get("type") == Some(&Value::String("null".into())));
                    if let Some(idx) = null_idx {
                        let type_schema = variants[1 - idx].clone();
                        if let Value::Object(type_map) = type_schema {
                            map.remove(key);
                            for (k, v) in type_map {
                                map.entry(&k).or_insert(v);
                            }
                            map.insert("nullable".into(), Value::Bool(true));
                            break;
                        }
                    }
                }
            }

            // type: ["string", "null"] → type: "string", nullable: true
            if let Some(Value::Array(types)) = map.get("type")
                && types.len() == 2
            {
                let null_idx = types.iter().position(|v| v.as_str() == Some("null"));
                if let Some(idx) = null_idx {
                    let actual_type = types[1 - idx].clone();
                    map.insert("type".into(), actual_type);
                    map.insert("nullable".into(), Value::Bool(true));
                }
            }

            for v in map.values_mut() {
                flatten_nullable(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                flatten_nullable(v);
            }
        }
        _ => {}
    }
}

/// Recursively remove named fields from all objects in the schema.
pub fn strip_fields(schema: &mut Value, fields: &[&str]) {
    match schema {
        Value::Object(map) => {
            for field in fields {
                map.remove(*field);
            }
            for v in map.values_mut() {
                strip_fields(v, fields);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                strip_fields(v, fields);
            }
        }
        _ => {}
    }
}
