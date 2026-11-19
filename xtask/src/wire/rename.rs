//! serde's `rename_all` rules, applied the way serde applies them.

/// Applies `rule` to a Rust field name (`snake_case`).
pub fn field(rule: &str, name: &str) -> Result<String, String> {
    Ok(match rule {
        "lowercase" | "snake_case" => name.to_string(),
        "UPPERCASE" | "SCREAMING_SNAKE_CASE" => name.to_uppercase(),
        "PascalCase" => pascal(name),
        "camelCase" => {
            let p = pascal(name);
            let mut cs = p.chars();
            cs.next().map(|f| f.to_lowercase().collect::<String>() + cs.as_str()).unwrap_or_default()
        }
        "kebab-case" => name.replace('_', "-"),
        "SCREAMING-KEBAB-CASE" => name.to_uppercase().replace('_', "-"),
        other => return Err(format!("serde rename_all `{other}` is not one this reads")),
    })
}

/// Applies `rule` to a Rust variant name (`PascalCase`).
pub fn variant(rule: &str, name: &str) -> Result<String, String> {
    Ok(match rule {
        "PascalCase" => name.to_string(),
        "lowercase" => name.to_lowercase(),
        "UPPERCASE" => name.to_uppercase(),
        "camelCase" => {
            let mut cs = name.chars();
            cs.next().map(|f| f.to_lowercase().collect::<String>() + cs.as_str()).unwrap_or_default()
        }
        "snake_case" => snake(name),
        "SCREAMING_SNAKE_CASE" => snake(name).to_uppercase(),
        "kebab-case" => snake(name).replace('_', "-"),
        "SCREAMING-KEBAB-CASE" => snake(name).to_uppercase().replace('_', "-"),
        other => return Err(format!("serde rename_all `{other}` is not one this reads")),
    })
}

fn pascal(name: &str) -> String {
    let mut out = String::new();
    let mut up = true;
    for ch in name.chars() {
        if ch == '_' {
            up = true;
        } else if up {
            out.extend(ch.to_uppercase());
            up = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.char_indices() {
        if i > 0 && ch.is_uppercase() {
            out.push('_');
        }
        out.extend(ch.to_lowercase());
    }
    out
}
