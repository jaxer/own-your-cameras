//! Boundary between bindgen layout assertions and the exported camera-size evidence.
use std::collections::BTreeMap;

pub fn extract_sizes(tests: &str) -> Result<BTreeMap<String, String>, String> {
    // `["Size of NAME"][::core::mem::size_of::<NAME>() - 8usize];`, which rustfmt may wrap.
    let size_re = regex::Regex::new(
        r#"\["Size of (\w+)"\]\s*\[::core::mem::size_of::<(\w+)>\(\) - (\d+)usize\]"#,
    )
    .expect("regex");
    let mut sizes = std::collections::BTreeMap::new();
    for cap in size_re.captures_iter(tests) {
        if cap[1] != cap[2]
            || sizes
                .insert(cap[1].to_string(), cap[3].to_string())
                .is_some()
        {
            return Err(format!("ambiguous size evidence for {}", &cap[1]));
        }
    }
    if sizes.is_empty() {
        return Err("bindgen emitted no recognized record sizes".into());
    }
    // Generic bindgen helpers have no concrete C layout. Every concrete record in the
    // wrapper closure must have evidence; accepting a partial table hides generator drift.
    let record_re = regex::Regex::new(r"pub (?:struct|union) (\w+)\s*\{").expect("regex");
    for cap in record_re.captures_iter(tests) {
        if !sizes.contains_key(&cap[1]) {
            return Err(format!("missing size evidence for {}", &cap[1]));
        }
    }
    // `pub type HD_FOO = _HD_FOO;` gives the typedef name the same size.
    let alias_re = regex::Regex::new(r"pub type (\w+)\s*=\s*(\w+)\s*;").expect("regex");
    let aliases: Vec<_> = alias_re
        .captures_iter(tests)
        .map(|cap| (cap[1].to_string(), cap[2].to_string()))
        .collect();
    loop {
        let before = sizes.len();
        for (alias, target) in &aliases {
            if let Some(size) = sizes.get(target).cloned() {
                sizes.entry(alias.clone()).or_insert(size);
            }
        }
        if sizes.len() == before {
            break;
        }
    }
    Ok(sizes)
}
