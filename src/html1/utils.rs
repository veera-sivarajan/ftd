pub fn trim_all_lines(s: &str) -> String {
    use itertools::Itertools;

    s.split('\n').into_iter().map(|v| v.trim()).join("\n")
}

pub fn trim_start_once(s: &str, matches: &str) -> String {
    if let Some((_, p2)) = s.split_once(matches) {
        return p2.to_string();
    }
    s.to_string()
}

pub fn trim_end_once(s: &str, matches: &str) -> String {
    if let Some((p1, _)) = s.rsplit_once(matches) {
        return p1.to_string();
    }
    s.to_string()
}

pub fn trim_brackets(s: &str) -> String {
    if s.starts_with('(') && s.ends_with(')') {
        return s[1..s.len() - 1].to_string();
    }
    s.to_string()
}

pub(crate) fn name_with_id(s: &str, id: &str) -> String {
    if is_ftd_function(s) {
        return s.to_string();
    }
    format!("{}:{}", s, id)
}

pub(crate) fn is_ftd_function(s: &str) -> bool {
    s.starts_with("ftd#")
}

pub(crate) fn function_name_to_js_function(s: &str) -> String {
    s.replace('#', "__")
        .replace('-', "_")
        .replace(':', "___")
        .replace(',', "$")
}

pub(crate) fn full_data_id(id: &str, data_id: &str) -> String {
    if data_id.trim().is_empty() {
        id.to_string()
    } else {
        format!("{}:{}", data_id, id)
    }
}

pub(crate) fn get_formatted_dep_string_from_property_value(
    id: &str,
    doc: &ftd::interpreter2::TDoc,
    property_value: &ftd::interpreter2::PropertyValue,
    pattern: &Option<String>,
    field: Option<String>,
) -> ftd::html1::Result<Option<String>> {
    let value_string = match property_value {
        ftd::interpreter2::PropertyValue::Reference { name, .. } => {
            format!(
                "data[\"{}\"]{}",
                name,
                field.map(|v| format!(".{}", v)).unwrap_or("".to_string())
            )
        }
        ftd::interpreter2::PropertyValue::FunctionCall(function_call) => {
            let action = serde_json::to_string(&ftd::html1::Action::from_function_call(
                function_call,
                id,
                doc,
            )?)
            .unwrap();
            format!(
                "window.ftd.handle_function(event, '{}', '{}', this)",
                id, action
            )
        }
        ftd::interpreter2::PropertyValue::Value {
            value, line_number, ..
        } => value.to_string(doc, *line_number, field)?,
        _ => return Ok(None),
    };

    Ok(Some(match pattern {
        Some(p) => format!("\"{}\".format({})", p, value_string),
        None => value_string,
    }))
}

pub(crate) fn get_condition_string(condition: &ftd::interpreter2::Expression) -> String {
    let node = condition
        .expression
        .update_node_with_variable_reference(&condition.references);
    let expression = ftd::html1::ExpressionGenerator.to_string(&node, true, &[]);
    format!(
        indoc::indoc! {"
                function(){{
                    {expression}
                }}()"
        },
        expression = expression.trim(),
    )
}

pub(crate) fn js_expression_from_list(expressions: Vec<(Option<String>, String)>) -> String {
    let mut conditions = vec![];
    let mut default = None;
    for (condition, expression) in expressions {
        if let Some(condition) = condition {
            conditions.push(format!(
                indoc::indoc! {"
                        {if_exp}({condition}){{
                            {expression}
                        }}
                    "},
                if_exp = if conditions.is_empty() {
                    "if"
                } else {
                    "else if"
                },
                condition = condition,
                expression = expression.trim(),
            ));
        } else {
            default = Some(expression)
        }
    }

    let default = match default {
        Some(d) if conditions.is_empty() => d,
        Some(d) => format!("else {{{}}}", d),
        None => "".to_string(),
    };

    format!(
        indoc::indoc! {"
            {expressions}{default}
        "},
        expressions = conditions.join(" "),
        default = default,
    )
}

pub(crate) fn is_dark_mode_dependent(
    value: &ftd::interpreter2::PropertyValue,
    doc: &ftd::interpreter2::TDoc,
) -> ftd::html1::Result<bool> {
    let value = value.clone().resolve(doc, value.line_number())?;
    Ok(value.is_record("ftd#image-src"))
}
