use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Lit(String),
    Var(String),
    EachStart(String),
    EachEnd,
    IfStart(String),
    IfEnd,
    Else,
}

fn tokenize(tpl: &str) -> Result<Vec<Tok>, String> {
    let mut toks = Vec::new();
    let mut rest = tpl;
    while let Some(start) = rest.find("{{") {
        let (lit, after) = rest.split_at(start);
        if !lit.is_empty() {
            toks.push(Tok::Lit(lit.to_string()));
        }
        let after = &after[2..];
        match after.find("}}") {
            Some(end) => {
                let tag = after[..end].trim();
                let after = &after[end + 2..];
                if let Some(name) = tag.strip_prefix("#each ") {
                    toks.push(Tok::EachStart(name.trim().to_string()));
                } else if let Some(name) = tag.strip_prefix("#if ") {
                    toks.push(Tok::IfStart(name.trim().to_string()));
                } else if tag == "/each" {
                    toks.push(Tok::EachEnd);
                } else if tag == "/if" {
                    toks.push(Tok::IfEnd);
                } else if tag == "else" {
                    toks.push(Tok::Else);
                } else if tag.is_empty() || tag.starts_with(['#', '/']) {
                    return Err(format!("invalid template tag: {{{{{tag}}}}}"));
                } else {
                    toks.push(Tok::Var(tag.to_string()));
                }
                rest = after;
            }
            None => {
                return Err("unclosed template tag".into());
            }
        }
    }
    if !rest.is_empty() {
        toks.push(Tok::Lit(rest.to_string()));
    }
    Ok(toks)
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn resolve<'a>(path: &str, stack: &[&'a Value]) -> Result<&'a Value, String> {
    let missing = || format!("missing template variable: {path}");
    let mut parts = path.split('.');
    let head = parts.next().ok_or_else(missing)?;
    let mut cur = match head {
        "this" => stack.last().copied(),
        "root" => stack.first().copied(),
        _ => stack.iter().rev().find_map(|v| v.get(head)),
    }
    .ok_or_else(missing)?;
    for part in parts {
        cur = cur.get(part).ok_or_else(missing)?;
    }
    Ok(cur)
}

// Validate all branches before rendering, including branches that will be skipped.
fn validate(toks: &[Tok]) -> Result<(), String> {
    let mut blocks = Vec::new();
    for tok in toks {
        match tok {
            Tok::IfStart(path) | Tok::EachStart(path) => {
                if path.is_empty() {
                    return Err("template block requires a variable".into());
                }
                blocks.push((matches!(tok, Tok::IfStart(_)), false));
            }
            Tok::Else => match blocks.last_mut() {
                Some((true, seen_else)) if !*seen_else => *seen_else = true,
                _ => return Err("unexpected template else".into()),
            },
            Tok::IfEnd | Tok::EachEnd => {
                let expected_if = matches!(tok, Tok::IfEnd);
                match blocks.pop() {
                    Some((is_if, _)) if is_if == expected_if => {}
                    _ => {
                        return Err(format!(
                            "unexpected template closing tag: {}",
                            if expected_if { "/if" } else { "/each" }
                        ))
                    }
                }
            }
            _ => {}
        }
    }
    if let Some((is_if, _)) = blocks.last() {
        return Err(format!(
            "unclosed template block: {}",
            if *is_if { "if" } else { "each" }
        ));
    }
    Ok(())
}

fn scan_stop(toks: &[Tok], mut i: usize, stops: &[&str]) -> (usize, &'static str) {
    let mut if_d = 0i32;
    let mut each_d = 0i32;
    while i < toks.len() {
        match &toks[i] {
            Tok::IfStart(_) => if_d += 1,
            Tok::EachStart(_) => each_d += 1,
            Tok::IfEnd => {
                if if_d == 0 && each_d == 0 && stops.contains(&"/if") {
                    return (i, "/if");
                }
                if_d = (if_d - 1).max(0);
            }
            Tok::EachEnd => {
                if if_d == 0 && each_d == 0 && stops.contains(&"/each") {
                    return (i, "/each");
                }
                each_d = (each_d - 1).max(0);
            }
            Tok::Else if if_d == 0 && each_d == 0 && stops.contains(&"else") => {
                return (i, "else");
            }
            _ => {}
        }
        i += 1;
    }
    (toks.len(), "")
}

fn render_until(
    toks: &[Tok],
    i: &mut usize,
    stack: &mut Vec<&Value>,
    out: &mut String,
) -> Result<(usize, &'static str), String> {
    while *i < toks.len() {
        match &toks[*i] {
            Tok::Lit(s) => {
                out.push_str(s);
                *i += 1;
            }
            Tok::Var(name) => {
                match resolve(name, stack)? {
                    Value::Null => {}
                    Value::String(s) => out.push_str(s),
                    v => out.push_str(&v.to_string()),
                }
                *i += 1;
            }
            Tok::IfStart(name) => {
                let truthy = is_truthy(resolve(name, stack)?);
                *i += 1;
                if truthy {
                    let (j, kind) = render_until(toks, i, stack, out)?;
                    if kind == "else" {
                        let (j2, _) = scan_stop(toks, j + 1, &["/if"]);
                        *i = j2 + 1;
                    } else {
                        *i = j + 1;
                    }
                } else {
                    let (j, kind) = scan_stop(toks, *i, &["else", "/if"]);
                    if kind == "else" {
                        *i = j + 1;
                        let (j2, _) = render_until(toks, i, stack, out)?;
                        *i = j2 + 1;
                    } else {
                        *i = j + 1;
                    }
                }
            }
            Tok::EachStart(name) => {
                let start_i = *i + 1;
                let items = resolve(name, stack)?
                    .as_array()
                    .ok_or_else(|| format!("template each variable is not an array: {name}"))?;
                let mut end_i = start_i;
                if items.is_empty() {
                    let (j, _) = scan_stop(toks, start_i, &["/each"]);
                    end_i = j;
                } else {
                    for it in items {
                        stack.push(it);
                        *i = start_i;
                        let (j, _) = render_until(toks, i, stack, out)?;
                        stack.pop();
                        end_i = j;
                    }
                }
                *i = end_i + 1;
            }
            Tok::EachEnd => return Ok((*i, "/each")),
            Tok::IfEnd => return Ok((*i, "/if")),
            Tok::Else => return Ok((*i, "else")),
        }
    }
    Ok((toks.len(), ""))
}

pub fn render(tpl: &str, ctx: &Value) -> Result<String, String> {
    let toks = tokenize(tpl)?;
    validate(&toks)?;
    let mut out = String::new();
    let mut i = 0usize;
    let mut stack = vec![ctx];
    render_until(&toks, &mut i, &mut stack, &mut out)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn basic() {
        let ctx = json!({"topic":"tea", "items":[{"t":"a"},{"t":"b"}], "n": 0});
        assert_eq!(render("Hello {{topic}}!", &ctx).unwrap(), "Hello tea!");
        assert_eq!(
            render("{{#each items}}[{{this.t}}]{{/each}}", &ctx).unwrap(),
            "[a][b]"
        );
        assert_eq!(render("{{#if n}}yes{{else}}no{{/if}}", &ctx).unwrap(), "no");
        assert_eq!(
            render("{{#if topic}}yes{{else}}no{{/if}}", &ctx).unwrap(),
            "yes"
        );
    }
    #[test]
    fn rejects_malformed_templates_even_in_skipped_branches() {
        let ctx = json!({"enabled": false, "items": []});
        for tpl in [
            "before{{/if}}after",
            "{{/each}}",
            "{{else}}",
            "{{topic",
            "{{}}",
            "{{#unknown x}}",
            "{{#if enabled}}unclosed",
            "{{#each items}}unclosed",
            "{{#if enabled}}{{/each}}",
            "{{#each items}}{{/if}}",
            "{{#if enabled}}{{#each items}}{{/if}}{{/each}}",
            "{{#if enabled}}a{{else}}b{{else}}c{{/if}}",
            "{{#each items}}{{else}}{{/each}}",
        ] {
            assert!(render(tpl, &ctx).is_err(), "accepted {tpl}");
        }
    }

    #[test]
    fn missing_variables_and_invalid_loop_types_are_errors() {
        let ctx = json!({"obj": {}, "items": [{}], "null": null});
        for tpl in [
            "{{missing}}",
            "{{obj.missing}}",
            "{{#if missing}}yes{{/if}}",
            "{{#each missing}}item{{/each}}",
            "{{#each obj}}item{{/each}}",
            "{{#each items}}{{this.missing}}{{/each}}",
        ] {
            assert!(render(tpl, &ctx).is_err(), "accepted {tpl}");
        }
        assert_eq!(
            render("{{missing}}", &ctx).unwrap_err(),
            "missing template variable: missing"
        );
        assert_eq!(render("{{null}}", &ctx).unwrap(), "");
    }

    #[test]
    fn nested_blocks_preserve_scope_and_skip_unselected_values() {
        let ctx = json!({"topic": "tea", "groups": [
            {"name": "A", "enabled": true, "items": ["x", "y"]},
            {"name": "B", "enabled": false, "items": []}
        ]});
        let tpl = "{{#each groups}}{{#if enabled}}{{#each items}}{{root.topic}}/{{name}}/{{this}};{{/each}}{{else}}{{#if items}}{{missing}}{{else}}empty{{/if}}{{/if}}{{/each}}!";
        assert_eq!(render(tpl, &ctx).unwrap(), "tea/A/x;tea/A/y;empty!");
        assert_eq!(
            render(
                "{{#each groups}}{{#each this.items}}{{this}}{{/each}}{{/each}}tail",
                &ctx
            )
            .unwrap(),
            "xytail"
        );
    }

    #[test]
    fn substitutions_are_literal_and_structured_values_are_json() {
        let ctx = json!({"text": "é {{missing}} {{/if}}", "obj": {"a": 1}, "array": [true, 2]});
        assert_eq!(
            render("{{text}}|{{obj}}|{{array}}", &ctx).unwrap(),
            "é {{missing}} {{/if}}|{\"a\":1}|[true,2]"
        );
    }

    #[test]
    fn shipped_prompts_match_pipeline_contexts() {
        use crate::prompts::{USER_CRITIQUE, USER_FIX, USER_HUMAN_FIX, USER_RESEARCH};
        let base = json!({"topic": "TOPIC", "summary": "SUMMARY"});
        for (tpl, extra) in [
            (USER_RESEARCH, json!({})),
            (USER_CRITIQUE, json!({"bibliography": "BIBLIOGRAPHY"})),
            (
                USER_FIX,
                json!({"bibliography": "BIBLIOGRAPHY", "issues": "ISSUES"}),
            ),
            (
                USER_HUMAN_FIX,
                json!({"bibliography": "BIBLIOGRAPHY", "feedback": "FEEDBACK"}),
            ),
        ] {
            let mut ctx = base.clone();
            ctx.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            let rendered = render(tpl, &ctx).unwrap();
            for (key, value) in ctx.as_object().unwrap() {
                assert!(
                    rendered.contains(value.as_str().unwrap()),
                    "unused field {key}"
                );
                let mut missing = ctx.clone();
                missing.as_object_mut().unwrap().remove(key);
                assert!(render(tpl, &missing).is_err(), "missing {key} accepted");
            }
        }
    }
}
