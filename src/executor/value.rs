#[derive(serde::Deserialize, Debug, PartialEq, Default, Clone, serde::Serialize)]
pub struct Value<T> {
    pub value: T,
    pub line_number: Option<usize>,
    pub properties: Vec<ftd::interpreter2::Property>,
}

impl<T> Value<T> {
    pub fn new(
        value: T,
        line_number: Option<usize>,
        properties: Vec<ftd::interpreter2::Property>,
    ) -> Value<T> {
        Value {
            value,
            line_number,
            properties,
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Value<U> {
        Value {
            value: f(self.value),
            line_number: self.line_number,
            properties: self.properties,
        }
    }
}

pub(crate) fn get_value_from_properties_using_key_and_arguments(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<Option<ftd::interpreter2::Value>>> {
    let argument =
        arguments
            .iter()
            .find(|v| v.name.eq(key))
            .ok_or(ftd::executor::Error::ParseError {
                message: format!("Cannot find `{}` argument", key),
                doc_id: doc.name.to_string(),
                line_number,
            })?;

    let sources = argument.to_sources();
    let ftd::executor::Value {
        line_number: v_line_number,
        properties,
        value,
    } = find_value_by_argument(sources.as_slice(), properties, doc, argument, line_number)?;
    let expected_kind = value.as_ref().map(|v| v.kind());
    if !expected_kind
        .as_ref()
        .map_or(true, |v| v.is_same_as(&argument.kind.kind))
    {
        return ftd::executor::utils::parse_error(
            format!(
                "Expected kind {:?}, found: `{:?}`",
                expected_kind, argument.kind.kind
            ),
            doc.name,
            line_number,
        );
    }

    Ok(ftd::executor::Value::new(value, v_line_number, properties))
}

pub(crate) fn find_properties_by_source(
    source: &[ftd::interpreter2::PropertySource],
    properties: &[ftd::interpreter2::Property],
    doc: &ftd::executor::TDoc,
    argument: &ftd::interpreter2::Argument,
    line_number: usize,
) -> ftd::executor::Result<Vec<ftd::interpreter2::Property>> {
    use itertools::Itertools;

    let mut properties = properties
        .iter()
        .filter(|v| source.iter().any(|s| v.source.is_equal(s)))
        .map(ToOwned::to_owned)
        .collect_vec();

    ftd::executor::utils::validate_properties_and_set_default(
        &mut properties,
        argument,
        doc.name,
        line_number,
    )?;

    Ok(properties)
}

pub(crate) fn find_value_by_argument(
    source: &[ftd::interpreter2::PropertySource],
    properties: &[ftd::interpreter2::Property],
    doc: &ftd::executor::TDoc,
    argument: &ftd::interpreter2::Argument,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<Option<ftd::interpreter2::Value>>> {
    let properties = ftd::executor::value::find_properties_by_source(
        source,
        properties,
        doc,
        argument,
        line_number,
    )?;

    let mut value = None;
    let mut line_number = None;
    for p in properties.iter() {
        if let Some(v) = p.resolve(&doc.itdoc())? {
            value = Some(v);
            line_number = Some(p.line_number);
            if p.condition.is_some() {
                break;
            }
        }
    }

    Ok(ftd::executor::Value::new(value, line_number, properties))
}

pub fn string_list(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<Vec<String>>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::List { data, kind }) if kind.is_string() => {
            let mut values = vec![];
            for d in data {
                values.push(
                    d.resolve(&doc.itdoc(), line_number)?
                        .string(doc.name, line_number)?,
                );
            }
            Ok(ftd::executor::Value::new(
                values,
                value.line_number,
                value.properties,
            ))
        }
        None => Ok(ftd::executor::Value::new(
            vec![],
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type string list, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

#[allow(dead_code)]
pub fn string(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<String>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::String { text }) => Ok(ftd::executor::Value::new(
            text,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type string, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

pub fn record(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
    rec_name: &str,
) -> ftd::executor::Result<ftd::executor::Value<ftd::Map<ftd::interpreter2::PropertyValue>>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::Record { name, fields }) if name.eq(rec_name) => Ok(
            ftd::executor::Value::new(fields, value.line_number, value.properties),
        ),
        t => ftd::executor::utils::parse_error(
            format!(
                "Expected value of type record `{}`, found: {:?}",
                rec_name, t
            ),
            doc.name,
            line_number,
        ),
    }
}

pub fn i64(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<i64>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::Integer { value: v }) => Ok(ftd::executor::Value::new(
            v,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type integer, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

pub fn f64(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<f64>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::Decimal { value: v }) => Ok(ftd::executor::Value::new(
            v,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type decimal, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

pub fn bool(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<bool>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::Boolean { value: v }) => Ok(ftd::executor::Value::new(
            v,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type boolean, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

pub fn bool_with_default(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    default: bool,
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<bool>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::Boolean { value: b }) => Ok(ftd::executor::Value::new(
            b,
            value.line_number,
            value.properties,
        )),
        None => Ok(ftd::executor::Value::new(
            default,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type optional bool, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

#[allow(dead_code)]
pub fn optional_i64(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<Option<i64>>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::Integer { value: v }) => Ok(ftd::executor::Value::new(
            Some(v),
            value.line_number,
            value.properties,
        )),
        None => Ok(ftd::executor::Value::new(
            None,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type optional integer, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

pub fn string_with_default(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    default: &str,
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<String>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::String { text }) => Ok(ftd::executor::Value::new(
            text,
            value.line_number,
            value.properties,
        )),
        None => Ok(ftd::executor::Value::new(
            default.to_string(),
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type optional string, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

pub fn optional_string(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<Option<String>>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::String { text }) => Ok(ftd::executor::Value::new(
            Some(text),
            value.line_number,
            value.properties,
        )),
        None => Ok(ftd::executor::Value::new(
            None,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type optional string, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

pub fn optional_bool(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<Option<bool>>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::Boolean { value: v }) => Ok(ftd::executor::Value::new(
            Some(v),
            value.line_number,
            value.properties,
        )),
        None => Ok(ftd::executor::Value::new(
            None,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type optional boolean, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

#[allow(dead_code)]
pub fn optional_f64(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
) -> ftd::executor::Result<ftd::executor::Value<Option<f64>>> {
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::Decimal { value: v }) => Ok(ftd::executor::Value::new(
            Some(v),
            value.line_number,
            value.properties,
        )),
        None => Ok(ftd::executor::Value::new(
            None,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!("Expected value of type optional decimal, found: {:?}", t),
            doc.name,
            line_number,
        ),
    }
}

pub fn optional_record(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
    rec_name: &str,
) -> ftd::executor::Result<ftd::executor::Value<Option<ftd::Map<ftd::interpreter2::PropertyValue>>>>
{
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::Record { name, fields }) if name.eq(rec_name) => Ok(
            ftd::executor::Value::new(Some(fields), value.line_number, value.properties),
        ),
        None => Ok(ftd::executor::Value::new(
            None,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!(
                "Expected value of type record `{}`, found: {:?}",
                rec_name, t
            ),
            doc.name,
            line_number,
        ),
    }
}

pub fn optional_or_type(
    key: &str,
    properties: &[ftd::interpreter2::Property],
    arguments: &[ftd::interpreter2::Argument],
    doc: &ftd::executor::TDoc,
    line_number: usize,
    rec_name: &str,
) -> ftd::executor::Result<ftd::executor::Value<Option<(String, ftd::interpreter2::PropertyValue)>>>
{
    let value = get_value_from_properties_using_key_and_arguments(
        key,
        properties,
        arguments,
        doc,
        line_number,
    )?;

    match value.value.and_then(|v| v.inner()) {
        Some(ftd::interpreter2::Value::OrType {
            name,
            value: property_value,
            variant,
            ..
        }) if name.eq(rec_name) => Ok(ftd::executor::Value::new(
            Some((variant, property_value.as_ref().to_owned())),
            value.line_number,
            value.properties,
        )),
        None => Ok(ftd::executor::Value::new(
            None,
            value.line_number,
            value.properties,
        )),
        t => ftd::executor::utils::parse_error(
            format!(
                "Expected value of type or-type `{}`, found: {:?}",
                rec_name, t
            ),
            doc.name,
            line_number,
        ),
    }
}
