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
    format!("{}:{}", s, id)
}

pub(crate) fn function_name_to_js_function(s: &str) -> String {
    s.replace('#', "__")
        .replace('-', "_")
        .replace(':', "___")
        .replace(',', "$")
        .replace("\\\\", "/")
        .replace('\\', "/")
        .replace(['/', '.'], "_")
}

pub(crate) fn js_reference_name(s: &str) -> String {
    s.replace("\\\\", "/").replace('\\', "/")
}

pub(crate) fn full_data_id(id: &str, data_id: &str) -> String {
    if data_id.trim().is_empty() {
        id.to_string()
    } else {
        format!("{}:{}", data_id, id)
    }
}

pub(crate) fn node_change_id(id: &str, attr: &str) -> String {
    format!("{}__{}", id, attr)
}

pub(crate) fn get_formatted_dep_string_from_property_value(
    id: &str,
    doc: &ftd::interpreter2::TDoc,
    property_value: &ftd::interpreter2::PropertyValue,
    pattern_with_eval: &Option<(String, bool)>,
    field: Option<String>,
    string_needs_no_quotes: bool,
) -> ftd::html1::Result<Option<String>> {
    /*let field = match field {
        None if property_value.kind().is_ftd_length()
            || property_value.kind().is_ftd_resizing_fixed() =>
        {
            Some("value".to_string())
        }
        Some(a) => Some(a),
        None => None,
    };*/

    let value_string = if let Some(value_string) =
        property_value.to_html_string(doc, field, id, string_needs_no_quotes)?
    {
        value_string
    } else {
        return Ok(None);
    };

    Ok(Some(match pattern_with_eval {
        Some((p, eval)) => {
            let mut pattern = format!("`{}`.format(JSONstringify({}))", p, value_string);
            if *eval {
                pattern = format!("eval({})", pattern)
            }
            pattern
        }
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

pub(crate) fn js_expression_from_list(
    expressions: Vec<(Option<String>, String)>,
    key: Option<&str>,
    default_for_null: &str,
) -> String {
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
        None if !conditions.is_empty() && key.is_some() && !default_for_null.is_empty() => {
            format!("else {{ {} }}", default_for_null)
        }
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
    Ok(value.is_record(ftd::interpreter2::FTD_IMAGE_SRC)
        || value.is_record(ftd::interpreter2::FTD_COLOR)
        || value.is_or_type_variant(ftd::interpreter2::FTD_BACKGROUND_SOLID))
}

pub(crate) fn is_device_dependent(
    value: &ftd::interpreter2::PropertyValue,
    doc: &ftd::interpreter2::TDoc,
) -> ftd::html1::Result<bool> {
    let value = value.clone().resolve(doc, value.line_number())?;
    Ok(value.is_record(ftd::interpreter2::FTD_RESPONSIVE_TYPE)
        || value.is_or_type_variant(ftd::interpreter2::FTD_LENGTH_RESPONSIVE))
}

pub(crate) fn dependencies_from_property_value(
    property_value: &ftd::interpreter2::PropertyValue,
    doc: &ftd::interpreter2::TDoc,
) -> Vec<String> {
    if let Some(ref_name) = property_value.reference_name() {
        vec![ref_name.to_string()]
    } else if let Some(function_call) = property_value.get_function() {
        let mut result = vec![];
        for property_value in function_call.values.values() {
            result.extend(dependencies_from_property_value(property_value, doc));
        }
        result
    } else if property_value.is_value() && property_value.kind().is_ftd_length() {
        dependencies_from_length_property_value(property_value, doc)
    } else if property_value.is_value() && property_value.kind().is_ftd_resizing_fixed() {
        let value = property_value.value("", 0).unwrap();
        let property_value = value
            .get_or_type(doc.name, property_value.line_number())
            .unwrap()
            .2;
        if property_value.is_value() && property_value.kind().is_ftd_length() {
            dependencies_from_length_property_value(property_value, doc)
        } else {
            vec![]
        }
    } else if property_value.is_value() && property_value.kind().is_ftd_responsive_type() {
        let value = property_value
            .value("", 0)
            .unwrap()
            .record_fields(doc.name, property_value.line_number())
            .unwrap();
        let mut values = vec![];
        for property_value in value.values() {
            if property_value.is_value() && property_value.kind().is_ftd_type() {
                let value = property_value
                    .value("", 0)
                    .unwrap()
                    .record_fields(doc.name, 0)
                    .unwrap();
                for property_value in value.values() {
                    if property_value.is_value() && property_value.kind().is_ftd_font_size() {
                        let value = property_value.value("", 0).unwrap();
                        let property_value = value.get_or_type(doc.name, 0).unwrap().2;
                        values.extend(dependencies_from_property_value(property_value, doc))
                    }
                }
            }
        }
        values
    } else {
        vec![]
    }
}

fn dependencies_from_length_property_value(
    property_value: &ftd::interpreter2::PropertyValue,
    doc: &ftd::interpreter2::TDoc,
) -> Vec<String> {
    if property_value.is_value() && property_value.kind().is_ftd_length() {
        let value = property_value
            .value(doc.name, property_value.line_number())
            .unwrap();
        if let Ok(property_value) = value.get_or_type(doc.name, property_value.line_number()) {
            dependencies_from_property_value(property_value.2, doc)
        } else if let Ok(property_value) =
            value.record_fields(doc.name, property_value.line_number())
        {
            let mut values = vec![];
            for field in property_value.values() {
                values.extend(dependencies_from_property_value(field, doc));
            }
            values
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

impl ftd::interpreter2::PropertyValue {
    pub(crate) fn to_html_string(
        &self,
        doc: &ftd::interpreter2::TDoc,
        field: Option<String>,
        id: &str,
        string_needs_no_quotes: bool,
    ) -> ftd::html1::Result<Option<String>> {
        Ok(match self {
            ftd::interpreter2::PropertyValue::Reference { name, .. } => Some(format!(
                "resolve_reference(\"{}\", data, null){}",
                js_reference_name(name),
                field
                    .map(|v| format!(".{}", v))
                    .unwrap_or_else(|| "".to_string())
            )),
            ftd::interpreter2::PropertyValue::FunctionCall(function_call) => {
                let action = serde_json::to_string(&ftd::html1::Action::from_function_call(
                    function_call,
                    id,
                    doc,
                )?)
                .unwrap();
                Some(format!(
                    "window.ftd.handle_function(event, '{}', '{}', this)",
                    id, action
                ))
            }
            ftd::interpreter2::PropertyValue::Value {
                value, line_number, ..
            } => value.to_html_string(doc, *line_number, field, id, string_needs_no_quotes)?,
            _ => None,
        })
    }
}

impl ftd::interpreter2::Value {
    // string_needs_no_quotes: for class attribute the value should be red-block not "red-block"
    pub(crate) fn to_html_string(
        &self,
        doc: &ftd::interpreter2::TDoc,
        line_number: usize,
        field: Option<String>,
        id: &str,
        string_needs_no_quotes: bool,
    ) -> ftd::html1::Result<Option<String>> {
        Ok(match self {
            ftd::interpreter2::Value::String { text } if !string_needs_no_quotes => {
                Some(format!("\"{}\"", text))
            }
            ftd::interpreter2::Value::String { text } if string_needs_no_quotes => {
                Some(text.to_string())
            }
            ftd::interpreter2::Value::Integer { value } => Some(value.to_string()),
            ftd::interpreter2::Value::Decimal { value } => Some(value.to_string()),
            ftd::interpreter2::Value::Boolean { value } => Some(value.to_string()),
            ftd::interpreter2::Value::List { data, .. } => {
                let mut values = vec![];
                for value in data {
                    let v = if let Some(v) =
                        value.clone().resolve(doc, line_number)?.to_html_string(
                            doc,
                            value.line_number(),
                            None,
                            id,
                            string_needs_no_quotes,
                        )? {
                        v
                    } else {
                        continue;
                    };
                    values.push(v);
                }
                Some(format!(
                    "{:?}",
                    values.join(if string_needs_no_quotes { " " } else { ", " })
                ))
            }
            ftd::interpreter2::Value::Record { fields, .. }
                if field
                    .as_ref()
                    .map(|v| fields.contains_key(v))
                    .unwrap_or(false) =>
            {
                fields.get(&field.unwrap()).unwrap().to_html_string(
                    doc,
                    None,
                    id,
                    string_needs_no_quotes,
                )?
            }
            ftd::interpreter2::Value::OrType {
                value,
                variant,
                full_variant,
                name,
                ..
            } => {
                let value = value.to_html_string(doc, field, id, string_needs_no_quotes)?;
                match value {
                    Some(value) if name.eq(ftd::interpreter2::FTD_LENGTH) => {
                        if let Ok(pattern) = ftd::executor::Length::set_pattern_from_variant_str(
                            variant,
                            doc.name,
                            line_number,
                        ) {
                            Some(format!("`{}`.format(JSONstringify({}))", pattern, value))
                        } else {
                            Some(value)
                        }
                    }
                    Some(value)
                        if name.eq(ftd::interpreter2::FTD_RESIZING)
                            && variant.ne(ftd::interpreter2::FTD_RESIZING_FIXED) =>
                    {
                        if let Ok(pattern) = ftd::executor::Resizing::set_pattern_from_variant_str(
                            variant,
                            full_variant,
                            doc.name,
                            line_number,
                        ) {
                            Some(format!("`{}`.format(JSONstringify({}))", pattern, value))
                        } else {
                            Some(value)
                        }
                    }
                    Some(value) => Some(value),
                    None => None,
                }
            }
            ftd::interpreter2::Value::Record { fields, .. } => {
                let mut values = vec![];
                for (k, v) in fields {
                    let value = if let Some(v) =
                        v.to_html_string(doc, field.clone(), id, string_needs_no_quotes)?
                    {
                        v
                    } else {
                        "null".to_string()
                    };
                    values.push(format!("\"{}\": {}", k, value));
                }

                Some(format!("{{{}}}", values.join(", ")))
            }
            t => unimplemented!("{:?}", t),
        })
    }
}

pub(crate) fn events_to_string(events: Vec<(String, String, String)>) -> String {
    use itertools::Itertools;

    if events.is_empty() {
        return "".to_string();
    }

    let global_variables =
        "let global_keys = {};\nlet buffer = [];\nlet lastKeyTime = Date.now();".to_string();
    let mut keydown_seq_event = "".to_string();
    let mut keydown_events = indoc::indoc! {"
        document.addEventListener(\"keydown\", function(event) {
            global_keys[event.key] = true;
            const currentTime = Date.now();
            if (currentTime - lastKeyTime > 1000) {{
                buffer = [];
            }}
            lastKeyTime = currentTime;
            if (event.target.nodeName === \"INPUT\" || event.target.nodeName === \"TEXTAREA\") {
                return;
            }          
            buffer.push(event.key);
    "}
    .to_string();

    for (keys, actions) in events.iter().filter_map(|e| {
        if let Some(keys) = e.1.strip_prefix("onglobalkeyseq[") {
            let keys = keys
                .trim_end_matches(']')
                .split('-')
                .map(to_key)
                .collect_vec();
            Some((keys, e.2.clone()))
        } else {
            None
        }
    }) {
        keydown_seq_event = format!(
            indoc::indoc! {"
                {string}
                if (buffer.join(',').includes(\"{sequence}\")) {{
                   {actions}
                    buffer = [];
                    global_keys[event.key] = false;
                    return;
                }}
            "},
            string = keydown_seq_event,
            sequence = keys.join(","),
            actions = actions,
        );
    }

    let keyup_events =
        "document.addEventListener(\"keyup\", function(event) { global_keys[event.key] = false; })"
            .to_string();

    for (keys, actions) in events.iter().filter_map(|e| {
        if let Some(keys) = e.1.strip_prefix("onglobalkey[") {
            let keys = keys
                .trim_end_matches(']')
                .split('-')
                .map(to_key)
                .collect_vec();
            Some((keys, e.2.clone()))
        } else {
            None
        }
    }) {
        let all_keys = keys
            .iter()
            .map(|v| format!("global_keys[\"{}\"]", v))
            .join(" && ");
        keydown_seq_event = format!(
            indoc::indoc! {"
                        {string}
                        if ({all_keys} && buffer.join(',').includes(\"{sequence}\")) {{
                            {actions}
                            buffer = [];
                            global_keys[event.key] = false;
                            return;
                        }}
                    "},
            string = keydown_seq_event,
            all_keys = all_keys,
            sequence = keys.join(","),
            actions = actions,
        );
    }

    if !keydown_seq_event.is_empty() {
        keydown_events = format!("{}\n\n{}}});", keydown_events, keydown_seq_event);
    }

    let mut string = "document.addEventListener(\"click\", function(event) {".to_string();
    for event in events.iter().filter(|e| e.1.eq("onclickoutside")) {
        string = format!(
            indoc::indoc! {"
                {string}
                if (document.querySelector(`[data-id=\"{data_id}\"]`).style.display !== \"none\" && !document.querySelector(`[data-id=\"{data_id}\"]`).contains(event.target)) {{
                    {event}
                }}
            "},
            string = string,
            data_id = event.0,
            event = event.2,
        );
    }
    string = format!("{}}});", string);

    if !keydown_seq_event.is_empty() {
        format!(
            "{}\n\n\n{}\n\n\n{}\n\n\n{}",
            string, global_variables, keydown_events, keyup_events
        )
    } else {
        string
    }
}

fn to_key(key: &str) -> String {
    match key {
        "ctrl" => "Control",
        "alt" => "Alt",
        "shift" => "Shift",
        "up" => "ArrowUp",
        "down" => "ArrowDown",
        "right" => "ArrowRight",
        "left" => "ArrowLeft",
        "esc" => "Escape",
        "dash" => "-",
        "space" => " ",
        t => t,
    }
    .to_string()
}
