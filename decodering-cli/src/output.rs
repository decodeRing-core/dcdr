use std::fmt::Write;

use console::style;
use serde_json::Value;

use crate::api::ApiResponse;

pub fn report(resp: &ApiResponse<Value>) {
    if !resp.message.is_empty() {
        let _ = cliclack::log::success(&resp.message);
    }
    if let Some(data) = &resp.data {
        let mut out = String::new();
        match data {
            Value::Object(_) | Value::Array(_) => {
                let _ = writeln!(out, "{}", style("Data").cyan().bold());
                render_tree(data, 1, &mut out);
            }
            scalar_value => render_tree(scalar_value, 0, &mut out),
        }
        let tree = out.trim_end();
        if !tree.is_empty() {
            let _ = cliclack::log::info(tree);
        }
    }
}

fn render_tree(value: &Value, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match value {
        Value::Object(map) => {
            if let Some(status) = status_variant(map) {
                let _ = writeln!(out, "{pad}{status}");
                return;
            }
            let key_w = map
                .iter()
                .filter(|(_, v)| is_inline(v))
                .map(|(k, _)| k.len())
                .max()
                .unwrap_or(0);
            for (k, v) in map {
                if is_inline(v) {
                    let key = style(format!("{k:<key_w$}")).dim();
                    let _ = writeln!(out, "{pad}{key}  {}", inline(v));
                } else {
                    let _ = writeln!(out, "{pad}{}", style(k).cyan().bold());
                    render_tree(v, indent + 1, out);
                }
            }
        }
        // Array / scalar arms unchanged
        Value::Array(arr) => {
            for v in arr {
                if is_inline(v) {
                    let _ = writeln!(out, "{pad}{} {}", style("-").dim(), inline(v));
                } else {
                    let _ = writeln!(out, "{pad}{}", style("-").dim());
                    render_tree(v, indent + 1, out);
                }
            }
        }
        other => {
            let _ = writeln!(out, "{pad}{}", scalar(other));
        }
    }
}

fn status_variant(map: &serde_json::Map<String, Value>) -> Option<String> {
    if map.len() != 1 {
        return None;
    }
    let (key, value) = map.iter().next()?;
    let label = match value {
        Value::Null => key.clone(),
        inner => format!("{key}: {}", inline(inner)),
    };
    match key.as_str() {
        "Ok" => Some(format!("{}", style(label).green())),
        "Err" | "Error" => Some(format!("{}", style(label).red())),
        _ => None,
    }
}

fn is_inline(value: &Value) -> bool {
    match value {
        Value::Array(arr) => arr.iter().all(|v| !v.is_object() && !v.is_array()),
        Value::Object(map) => map.is_empty(),
        _ => true,
    }
}

fn inline(value: &Value) -> String {
    match value {
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(scalar).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Object(_) => format!("{}", style("{}").dim()),
        other => scalar(other),
    }
}

fn scalar(value: &Value) -> String {
    match value {
        Value::Null => format!("{}", style("null").dim()),
        Value::String(s) => format!("{}", style(s).green()),
        Value::Bool(b) => format!("{}", style(b).yellow()),
        Value::Number(n) => format!("{}", style(n).yellow()),
        container => inline(container),
    }
}
