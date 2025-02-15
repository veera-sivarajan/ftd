#[derive(Debug, PartialEq)]
pub struct TDoc<'a> {
    pub name: &'a str,
    pub aliases: &'a ftd::Map<String>,
    pub bag: BagOrState<'a>,
}

#[derive(Debug, PartialEq)]
pub enum BagOrState<'a> {
    Bag(&'a ftd::Map<ftd::interpreter2::Thing>),
    State(&'a mut ftd::interpreter2::InterpreterState),
}

impl<'a> TDoc<'a> {
    pub fn new(
        name: &'a str,
        aliases: &'a ftd::Map<String>,
        bag: &'a ftd::Map<ftd::interpreter2::Thing>,
    ) -> TDoc<'a> {
        TDoc {
            name,
            aliases,
            bag: BagOrState::Bag(bag),
        }
    }

    pub fn new_state(
        name: &'a str,
        aliases: &'a ftd::Map<String>,
        state: &'a mut ftd::interpreter2::InterpreterState,
    ) -> TDoc<'a> {
        TDoc {
            name,
            aliases,
            bag: BagOrState::State(state),
        }
    }

    pub fn state(&'a self) -> Option<&&'a mut ftd::interpreter2::InterpreterState> {
        match &self.bag {
            BagOrState::Bag(_) => None,
            BagOrState::State(s) => Some(s),
        }
    }

    pub fn resolve_name(&self, name: &str) -> String {
        ftd::interpreter2::utils::resolve_name(name, self.name, self.aliases)
    }

    pub fn bag(&'a self) -> &'a ftd::Map<ftd::interpreter2::Thing> {
        match &self.bag {
            BagOrState::Bag(b) => b,
            BagOrState::State(s) => &s.bag,
        }
    }

    pub fn get_record(
        &'a self,
        name: &'a str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Record> {
        match self.get_thing(name, line_number)? {
            ftd::interpreter2::Thing::Record(r) => Ok(r),
            t => self.err(
                format!("Expected Record, found: `{:?}`", t).as_str(),
                name,
                "get_record",
                line_number,
            ),
        }
    }

    pub fn search_record(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::StateWithThing<ftd::interpreter2::Record>>
    {
        match self.search_thing(name, line_number)? {
            ftd::interpreter2::StateWithThing::State(s) => {
                Ok(ftd::interpreter2::StateWithThing::new_state(s))
            }
            ftd::interpreter2::StateWithThing::Continue => {
                Ok(ftd::interpreter2::StateWithThing::new_continue())
            }
            ftd::interpreter2::StateWithThing::Thing(ftd::interpreter2::Thing::Record(r)) => {
                Ok(ftd::interpreter2::StateWithThing::new_thing(r))
            }
            ftd::interpreter2::StateWithThing::Thing(t) => self.err(
                format!("Expected Record, found: `{:?}`", t).as_str(),
                name,
                "search_record",
                line_number,
            ),
        }
    }

    pub fn get_variable(
        &'a self,
        name: &'a str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Variable> {
        match self.get_thing(name, line_number)? {
            ftd::interpreter2::Thing::Variable(r) => Ok(r),
            t => self.err(
                format!("Expected Variable, found: `{:?}`", t).as_str(),
                name,
                "get_variable",
                line_number,
            ),
        }
    }

    pub fn get_value(
        &'a self,
        line_number: usize,
        name: &'a str,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Value> {
        // TODO: name can be a.b.c, and a and a.b are records with right fields
        match self.get_thing(name, line_number)? {
            ftd::interpreter2::Thing::Variable(v) => v.value.resolve(self, line_number),
            v => self.err("not a variable", v, "get_value", line_number),
        }
    }

    pub fn search_variable(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::StateWithThing<ftd::interpreter2::Variable>>
    {
        match self.search_thing(name, line_number)? {
            ftd::interpreter2::StateWithThing::State(s) => {
                Ok(ftd::interpreter2::StateWithThing::new_state(s))
            }
            ftd::interpreter2::StateWithThing::Continue => {
                Ok(ftd::interpreter2::StateWithThing::new_continue())
            }
            ftd::interpreter2::StateWithThing::Thing(ftd::interpreter2::Thing::Variable(r)) => {
                Ok(ftd::interpreter2::StateWithThing::new_thing(r))
            }
            ftd::interpreter2::StateWithThing::Thing(t) => self.err(
                format!("Expected Variable, found: `{:?}`", t).as_str(),
                name,
                "search_variable",
                line_number,
            ),
        }
    }

    pub fn eq(&'a self, name1: &'a str, name2: &'a str) -> bool {
        let name1 = self.resolve_name(name1);
        let name2 = self.resolve_name(name2);
        name1.eq(&name2)
    }

    pub(crate) fn resolve_reference_name(
        &self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<String> {
        Ok(if let Some(l) = name.strip_prefix('$') {
            let d =
                ftd::interpreter2::utils::get_doc_name_and_remaining(l, self.name, line_number).0;
            if ftd::interpreter2::utils::get_special_variable().contains(&d.as_str()) {
                return Ok(format!("${}", l));
            }
            format!("${}", self.resolve_name(l))
        } else {
            name.to_string()
        })
    }

    pub(crate) fn resolve(
        &self,
        name: &str,
        kind: &ftd::interpreter2::KindData,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Value> {
        let (value, _var_name, _var_line_number, remaining) =
            if let Ok(v) = self.get_initial_variable(name, line_number) {
                let mut value = v.0.value;
                for conditional in v.0.conditional_value.iter() {
                    if conditional.condition.eval(self)? {
                        value = conditional.value.clone();
                        break;
                    }
                }
                (value, v.0.name, v.0.line_number, v.1)
            } else if let Ok(v) = self.get_component(name, line_number) {
                (
                    ftd::interpreter2::PropertyValue::Value {
                        value: v.to_value(kind),
                        is_mutable: false,
                        line_number: v.line_number,
                    },
                    v.name,
                    v.line_number,
                    None,
                )
            } else {
                return ftd::interpreter2::utils::e2(
                    format!("Cannot find {} in get_thing", name),
                    self.name,
                    line_number,
                );
            };
        let value = value.resolve(self, line_number)?;
        if let Some(remaining) = remaining {
            return resolve_(remaining.as_str(), &value, line_number, self);
        }
        return Ok(value);

        fn resolve_(
            name: &str,
            value: &ftd::interpreter2::Value,
            line_number: usize,
            doc: &ftd::interpreter2::TDoc,
        ) -> ftd::interpreter2::Result<ftd::interpreter2::Value> {
            let (p1, p2) = ftd::interpreter2::utils::split_at(name, ".");
            match value {
                ftd::interpreter2::Value::Record {
                    name: rec_name,
                    fields,
                } => {
                    let field = fields
                        .get(p1.as_str())
                        .ok_or(ftd::interpreter2::Error::ParseError {
                            message: format!("Can't find field `{}` in record `{}`", p1, rec_name),
                            doc_id: doc.name.to_string(),
                            line_number,
                        })?
                        .clone()
                        .resolve(doc, line_number)?;
                    if let Some(p2) = p2 {
                        return resolve_(p2.as_str(), &field, line_number, doc);
                    }
                    Ok(field)
                }
                ftd::interpreter2::Value::List { data, kind } => {
                    let p1 = p1.parse::<usize>()?;
                    let value = data
                        .get(p1)
                        .ok_or(ftd::interpreter2::Error::ParseError {
                            message: format!(
                                "Can't find index `{}` in list of kind `{:?}`",
                                p1, kind
                            ),
                            doc_id: doc.name.to_string(),
                            line_number,
                        })?
                        .clone()
                        .resolve(doc, line_number)?;
                    if let Some(p2) = p2 {
                        return resolve_(p2.as_str(), &value, line_number, doc);
                    }
                    Ok(value)
                }
                t => ftd::interpreter2::utils::e2(
                    format!("Expected record found `{:?}`", t).as_str(),
                    doc.name,
                    line_number,
                ),
            }
        }
    }

    pub fn set_value(
        &'a self,
        name: &'a str,
        value: ftd::interpreter2::PropertyValue,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Variable> {
        let (mut variable, mut remaining) = self.get_initial_variable(name, line_number)?;

        if !variable.mutable {
            return ftd::interpreter2::utils::e2(
                format!(
                    "The variable declaration `{}` is not mutable in line number {}",
                    variable.name, variable.line_number
                )
                .as_str(),
                self.name,
                line_number,
            );
        }

        if let Some((var, rem)) =
            find_variable_reference(&variable.value, remaining.clone(), self, line_number)?
        {
            variable = var;
            remaining = rem;
        }

        set_value_(&mut variable, value, remaining, self, line_number)?;

        return Ok(variable.clone());

        fn find_variable_reference(
            value: &ftd::interpreter2::PropertyValue,
            name: Option<String>,
            doc: &ftd::interpreter2::TDoc,
            line_number: usize,
        ) -> ftd::interpreter2::Result<Option<(ftd::interpreter2::Variable, Option<String>)>>
        {
            let mut variable = None;
            let mut remaining = name;
            let mut value = value.clone();
            while let Some(reference) = value.reference_name() {
                let (var, rem) = doc.get_initial_variable(reference, line_number)?;
                value = var.value.clone();
                variable = Some(var);
                remaining = if let Some(remaining) = remaining {
                    Some(
                        rem.map(|v| format!("{}.{}", v, remaining))
                            .unwrap_or(remaining),
                    )
                } else {
                    rem
                };
            }

            if let ftd::interpreter2::PropertyValue::Clone { .. } = value {
                return Ok(variable.map(|v| (v, remaining)));
            }

            if let Some(ref remaining) = remaining {
                let (p1, p2) = ftd::interpreter2::utils::split_at(remaining, ".");
                let value = value.value(doc.name, line_number)?.inner().ok_or(
                    ftd::interpreter2::Error::ParseError {
                        message: format!(
                            "Value expected found null, `{:?}` in line number {}",
                            value, line_number
                        ),
                        doc_id: doc.name.to_string(),
                        line_number,
                    },
                )?;

                match value {
                    ftd::interpreter2::Value::Record {
                        name: rec_name,
                        fields,
                    } => {
                        let field_value = fields
                            .get(p1.as_str())
                            .ok_or(ftd::interpreter2::Error::ParseError {
                                message: format!(
                                    "Expected field {} in record `{}` in line number {}",
                                    p1, rec_name, line_number
                                ),
                                doc_id: doc.name.to_string(),
                                line_number,
                            })?
                            .to_owned();
                        if let Some(variable) =
                            find_variable_reference(&field_value, p2, doc, line_number)?
                        {
                            return Ok(Some(variable));
                        }
                    }
                    t => {
                        return ftd::interpreter2::utils::e2(
                            format!(
                                "Expected record, found `{:?}` in line number {}",
                                t, line_number
                            )
                            .as_str(),
                            doc.name,
                            line_number,
                        )
                    }
                }
            }

            Ok(variable.map(|v| (v, remaining)))
        }

        fn set_value_(
            variable: &mut ftd::interpreter2::Variable,
            value: ftd::interpreter2::PropertyValue,
            remaining: Option<String>,
            doc: &ftd::interpreter2::TDoc,
            line_number: usize,
        ) -> ftd::interpreter2::Result<()> {
            change_value(&mut variable.value, value, remaining, doc, line_number)?;
            Ok(())
        }

        fn change_value(
            value: &mut ftd::interpreter2::PropertyValue,
            set: ftd::interpreter2::PropertyValue,
            remaining: Option<String>,
            doc: &ftd::interpreter2::TDoc,
            line_number: usize,
        ) -> ftd::interpreter2::Result<()> {
            if let Some(remaining) = remaining {
                let (p1, p2) = ftd::interpreter2::utils::split_at(remaining.as_str(), ".");
                match value {
                    ftd::interpreter2::PropertyValue::Value { value, .. } => match value {
                        ftd::interpreter2::Value::Record { name, fields } => {
                            let field = fields.get_mut(p1.as_str()).ok_or(
                                ftd::interpreter2::Error::ParseError {
                                    message: format!(
                                        "Can't find field `{}` in record `{}`",
                                        p1, name
                                    ),
                                    doc_id: doc.name.to_string(),
                                    line_number,
                                },
                            )?;
                            change_value(field, set, p2, doc, line_number)?;
                        }
                        t => {
                            return ftd::interpreter2::utils::e2(
                                format!("Expected record, found `{:?}`", t).as_str(),
                                doc.name,
                                line_number,
                            )
                        }
                    },
                    ftd::interpreter2::PropertyValue::Reference {
                        name,
                        kind,
                        is_mutable,
                        ..
                    }
                    | ftd::interpreter2::PropertyValue::Clone {
                        name,
                        kind,
                        is_mutable,
                        ..
                    } => {
                        let resolved_value = doc.resolve(name, kind, line_number)?;
                        *value = ftd::interpreter2::PropertyValue::Value {
                            value: resolved_value,
                            line_number,
                            is_mutable: *is_mutable,
                        };
                        change_value(value, set, Some(remaining), doc, line_number)?;
                    }
                    ftd::interpreter2::PropertyValue::FunctionCall(
                        ftd::interpreter2::FunctionCall {
                            name,
                            kind,
                            is_mutable,
                            values,
                            ..
                        },
                    ) => {
                        let function = doc.get_function(name, line_number)?;
                        let resolved_value = function
                            .resolve(kind, values, doc, line_number)?
                            .ok_or(ftd::interpreter2::Error::ParseError {
                                message: format!(
                                    "Expected return value of type {:?} for function {}",
                                    kind, name
                                ),
                                doc_id: doc.name.to_string(),
                                line_number,
                            })?;
                        *value = ftd::interpreter2::PropertyValue::Value {
                            value: resolved_value,
                            line_number,
                            is_mutable: *is_mutable,
                        };
                        change_value(value, set, Some(remaining), doc, line_number)?;
                    }
                }
            } else if value.kind().inner().eq(&set.kind()) || value.kind().eq(&set.kind()) {
                *value = set;
            } else {
                return ftd::interpreter2::utils::e2(
                    format!(
                        "Expected kind `{:?}`, found: \
                    `{:?}`",
                        value.kind(),
                        set.kind()
                    ),
                    doc.name,
                    line_number,
                );
            }

            Ok(())
        }
    }

    pub fn get_kind_with_argument(
        &mut self,
        name: &str,
        line_number: usize,
        component_definition_name_with_arguments: Option<(&str, &[ftd::interpreter2::Argument])>,
        loop_object_name_and_kind: &Option<(String, ftd::interpreter2::Argument)>,
    ) -> ftd::interpreter2::Result<
        ftd::interpreter2::StateWithThing<(
            ftd::interpreter2::PropertyValueSource,
            ftd::interpreter2::KindData,
        )>,
    > {
        let name = name
            .strip_prefix(ftd::interpreter2::utils::REFERENCE)
            .or_else(|| name.strip_prefix(ftd::interpreter2::utils::CLONE))
            .unwrap_or(name);

        let initial_kind_with_remaining_and_source =
            ftd::interpreter2::utils::get_argument_for_reference_and_remaining(
                name,
                self,
                component_definition_name_with_arguments,
                loop_object_name_and_kind,
                line_number,
            )?
            .map(|v| (v.0.kind.to_owned(), v.1, v.2));

        let (initial_kind, remaining, source) =
            if let Some(r) = initial_kind_with_remaining_and_source {
                r
            } else {
                let (initial_thing, remaining) =
                    try_ok_state!(self.search_initial_thing(name, line_number)?);

                let initial_kind = match initial_thing {
                    ftd::interpreter2::Thing::Record(r) => {
                        ftd::interpreter2::Kind::record(r.name.as_str())
                            .into_kind_data()
                            .caption_or_body()
                    }
                    ftd::interpreter2::Thing::OrType(o) => {
                        ftd::interpreter2::Kind::or_type(o.name.as_str())
                            .into_kind_data()
                            .caption_or_body()
                    }
                    ftd::interpreter2::Thing::OrTypeWithVariant { or_type, variant } => {
                        ftd::interpreter2::Kind::or_type_with_variant(
                            or_type.as_str(),
                            variant.name().as_str(),
                            variant.name().as_str(),
                        )
                        .into_kind_data()
                        .caption_or_body()
                    }
                    ftd::interpreter2::Thing::Variable(v) => v.kind,
                    ftd::interpreter2::Thing::Component(c) => {
                        ftd::interpreter2::Kind::ui_with_name(c.name.as_str())
                            .into_kind_data()
                            .caption_or_body()
                    }
                    ftd::interpreter2::Thing::Function(_) => todo!(),
                };

                (
                    initial_kind,
                    remaining,
                    ftd::interpreter2::PropertyValueSource::Global,
                )
            };

        if let Some(remaining) = remaining {
            return Ok(ftd::interpreter2::StateWithThing::new_thing((
                source,
                try_ok_state!(get_kind_(
                    initial_kind.kind,
                    remaining.as_str(),
                    self,
                    line_number
                )?),
            )));
        }

        return Ok(ftd::interpreter2::StateWithThing::new_thing((
            source,
            initial_kind,
        )));

        fn get_kind_(
            kind: ftd::interpreter2::Kind,
            name: &str,
            doc: &mut ftd::interpreter2::TDoc,
            line_number: usize,
        ) -> ftd::interpreter2::Result<ftd::interpreter2::StateWithThing<ftd::interpreter2::KindData>>
        {
            let (v, remaining) = ftd::interpreter2::utils::split_at(name, ".");
            match kind {
                ftd::interpreter2::Kind::Record { name: rec_name } => {
                    let record = try_ok_state!(doc.search_record(rec_name.as_str(), line_number)?);
                    let field_kind = record.get_field(&v, doc.name, line_number)?.kind.to_owned();
                    if let Some(remaining) = remaining {
                        get_kind_(field_kind.kind, &remaining, doc, line_number)
                    } else {
                        Ok(ftd::interpreter2::StateWithThing::new_thing(field_kind))
                    }
                }
                ftd::interpreter2::Kind::List { kind } => {
                    if let Some(remaining) = remaining {
                        get_kind_(*kind, &remaining, doc, line_number)
                    } else {
                        Ok(ftd::interpreter2::StateWithThing::new_thing(
                            ftd::interpreter2::KindData::new(*kind),
                        ))
                    }
                }
                ftd::interpreter2::Kind::Optional { kind } => {
                    let state_with_thing = get_kind_(*kind, name, doc, line_number)?;
                    if let ftd::interpreter2::StateWithThing::Thing(ref t) = state_with_thing {
                        Ok(ftd::interpreter2::StateWithThing::new_thing(
                            t.to_owned().into_optional(),
                        ))
                    } else {
                        Ok(state_with_thing)
                    }
                }
                t => ftd::interpreter2::utils::e2(
                    format!("Expected Record field `{}`, found: `{:?}`", name, t),
                    doc.name,
                    line_number,
                ),
            }
        }
    }

    pub fn get_kind(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::StateWithThing<ftd::interpreter2::KindData>>
    {
        match self.get_kind_with_argument(name, line_number, None, &None)? {
            ftd::interpreter2::StateWithThing::State(s) => {
                Ok(ftd::interpreter2::StateWithThing::new_state(s))
            }
            ftd::interpreter2::StateWithThing::Continue => {
                Ok(ftd::interpreter2::StateWithThing::new_continue())
            }
            ftd::interpreter2::StateWithThing::Thing(fields) => {
                Ok(ftd::interpreter2::StateWithThing::new_thing(fields.1))
            }
        }
    }

    pub fn get_component(
        &'a self,
        name: &'a str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::ComponentDefinition> {
        match self.get_thing(name, line_number)? {
            ftd::interpreter2::Thing::Component(c) => Ok(c),
            t => self.err(
                format!("Expected Component, found: `{:?}`", t).as_str(),
                name,
                "get_component",
                line_number,
            ),
        }
    }

    pub fn search_component(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<
        ftd::interpreter2::StateWithThing<ftd::interpreter2::ComponentDefinition>,
    > {
        match self.search_thing(name, line_number)? {
            ftd::interpreter2::StateWithThing::State(s) => {
                Ok(ftd::interpreter2::StateWithThing::new_state(s))
            }
            ftd::interpreter2::StateWithThing::Continue => {
                Ok(ftd::interpreter2::StateWithThing::new_continue())
            }
            ftd::interpreter2::StateWithThing::Thing(ftd::interpreter2::Thing::Component(c)) => {
                Ok(ftd::interpreter2::StateWithThing::new_thing(c))
            }
            ftd::interpreter2::StateWithThing::Thing(t) => self.err(
                format!("Expected Component, found: `{:?}`", t).as_str(),
                name,
                "search_component",
                line_number,
            ),
        }
    }

    pub fn search_or_type(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::StateWithThing<ftd::interpreter2::OrType>>
    {
        match self.search_thing(name, line_number)? {
            ftd::interpreter2::StateWithThing::State(s) => {
                Ok(ftd::interpreter2::StateWithThing::new_state(s))
            }
            ftd::interpreter2::StateWithThing::Continue => {
                Ok(ftd::interpreter2::StateWithThing::new_continue())
            }
            ftd::interpreter2::StateWithThing::Thing(ftd::interpreter2::Thing::OrType(c)) => {
                Ok(ftd::interpreter2::StateWithThing::new_thing(c))
            }
            ftd::interpreter2::StateWithThing::Thing(t) => self.err(
                format!("Expected OrType, found: `{:?}`", t).as_str(),
                name,
                "search_or_type",
                line_number,
            ),
        }
    }

    pub fn search_or_type_with_variant(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<
        ftd::interpreter2::StateWithThing<(String, ftd::interpreter2::OrTypeVariant)>,
    > {
        match self.search_thing(name, line_number)? {
            ftd::interpreter2::StateWithThing::State(s) => {
                Ok(ftd::interpreter2::StateWithThing::new_state(s))
            }
            ftd::interpreter2::StateWithThing::Continue => {
                Ok(ftd::interpreter2::StateWithThing::new_continue())
            }
            ftd::interpreter2::StateWithThing::Thing(
                ftd::interpreter2::Thing::OrTypeWithVariant { or_type, variant },
            ) => Ok(ftd::interpreter2::StateWithThing::new_thing((
                or_type, variant,
            ))),
            ftd::interpreter2::StateWithThing::Thing(t) => self.err(
                format!("Expected OrTypeWithVariant, found: `{:?}`", t).as_str(),
                name,
                "search_or_type_with_variant",
                line_number,
            ),
        }
    }

    pub fn get_thing(
        &'a self,
        name: &'a str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Thing> {
        let name = name
            .strip_prefix(ftd::interpreter2::utils::REFERENCE)
            .or_else(|| name.strip_prefix(ftd::interpreter2::utils::CLONE))
            .unwrap_or(name);

        let (initial_thing, remaining) = self.get_initial_thing(name, line_number)?;

        if let Some(remaining) = remaining {
            return get_thing_(self, line_number, remaining.as_str(), &initial_thing);
        }
        return Ok(initial_thing);

        fn get_thing_(
            doc: &ftd::interpreter2::TDoc,
            line_number: usize,
            name: &str,
            thing: &ftd::interpreter2::Thing,
        ) -> ftd::interpreter2::Result<ftd::interpreter2::Thing> {
            let (v, remaining) = ftd::interpreter2::utils::split_at(name, ".");
            let thing = match thing.clone() {
                ftd::interpreter2::Thing::Variable(ftd::interpreter2::Variable {
                    name,
                    value,
                    mutable,
                    ..
                }) => {
                    let value_kind = value.kind();
                    let fields = match value.resolve(doc, line_number)?.inner() {
                        Some(ftd::interpreter2::Value::Record { fields, .. }) => fields,
                        Some(ftd::interpreter2::Value::Object { values }) => values,
                        Some(ftd::interpreter2::Value::List { data, .. }) => data
                            .into_iter()
                            .enumerate()
                            .map(|(index, v)| (index.to_string(), v))
                            .collect::<ftd::Map<ftd::interpreter2::PropertyValue>>(),
                        None => {
                            let kind_name = match value_kind.get_record_name() {
                                Some(name) => name,
                                _ => {
                                    return doc.err(
                                        "not an record",
                                        thing,
                                        "get_thing",
                                        line_number,
                                    );
                                }
                            };
                            let kind_thing = doc.get_thing(kind_name, line_number)?;
                            let kind = match kind_thing
                                .record(doc.name, line_number)?
                                .fields
                                .iter()
                                .find(|f| f.name.eq(&v))
                                .map(|v| v.kind.to_owned())
                            {
                                Some(f) => f,
                                _ => {
                                    return doc.err(
                                        "not an record or or-type",
                                        thing,
                                        "get_thing",
                                        line_number,
                                    );
                                }
                            };
                            let thing =
                                ftd::interpreter2::Thing::Variable(ftd::interpreter2::Variable {
                                    name,
                                    kind: kind.to_owned(),
                                    mutable,
                                    value: ftd::interpreter2::PropertyValue::Value {
                                        value: ftd::interpreter2::Value::Optional {
                                            data: Box::new(None),
                                            kind,
                                        },
                                        is_mutable: mutable,
                                        line_number,
                                    },
                                    conditional_value: vec![],
                                    line_number,
                                    is_static: !mutable,
                                });
                            if let Some(remaining) = remaining {
                                return get_thing_(doc, line_number, &remaining, &thing);
                            }
                            return Ok(thing);
                        }
                        _ => return doc.err("not an record", thing, "get_thing", line_number),
                    };
                    match fields.get(&v) {
                        Some(ftd::interpreter2::PropertyValue::Value {
                            value: val,
                            line_number,
                            is_mutable,
                        }) => ftd::interpreter2::Thing::Variable(ftd::interpreter2::Variable {
                            name,
                            kind: ftd::interpreter2::KindData {
                                kind: val.kind(),
                                caption: false,
                                body: false,
                            },
                            mutable: false,
                            value: ftd::interpreter2::PropertyValue::Value {
                                value: val.to_owned(),
                                line_number: *line_number,
                                is_mutable: *is_mutable,
                            },
                            conditional_value: vec![],
                            line_number: *line_number,
                            is_static: !mutable,
                        }),
                        Some(ftd::interpreter2::PropertyValue::Reference { name, .. })
                        | Some(ftd::interpreter2::PropertyValue::Clone { name, .. }) => {
                            let (initial_thing, name) = doc.get_initial_thing(name, line_number)?;
                            if let Some(remaining) = name {
                                get_thing_(doc, line_number, remaining.as_str(), &initial_thing)?
                            } else {
                                initial_thing
                            }
                        }
                        _ => thing.clone(),
                    }
                }
                _ => {
                    return doc.err("not an or-type", thing, "get_thing", line_number);
                }
            };
            if let Some(remaining) = remaining {
                return get_thing_(doc, line_number, &remaining, &thing);
            }
            Ok(thing)
        }
    }

    pub fn get_function(
        &'a self,
        name: &'a str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Function> {
        let initial_thing = self.get_initial_thing(name, line_number)?.0;
        initial_thing.function(self.name, line_number)
    }

    pub fn search_function(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::StateWithThing<ftd::interpreter2::Function>>
    {
        let initial_thing = try_ok_state!(self.search_initial_thing(name, line_number)?).0;
        Ok(ftd::interpreter2::StateWithThing::new_thing(
            initial_thing.function(self.name, line_number)?,
        ))
    }

    pub fn get_initial_variable(
        &'a self,
        name: &'a str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<(ftd::interpreter2::Variable, Option<String>)> {
        let (initial_thing, remaining) = self.get_initial_thing(name, line_number)?;
        Ok((initial_thing.variable(self.name, line_number)?, remaining))
    }

    pub fn scan_thing(&mut self, name: &str, line_number: usize) -> ftd::interpreter2::Result<()> {
        let name = name
            .strip_prefix(ftd::interpreter2::utils::REFERENCE)
            .or_else(|| name.strip_prefix(ftd::interpreter2::utils::CLONE))
            .unwrap_or(name);

        self.scan_initial_thing(name, line_number)
    }

    pub fn scan_initial_thing(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<()> {
        use itertools::Itertools;

        let name = name
            .strip_prefix(ftd::interpreter2::utils::REFERENCE)
            .or_else(|| name.strip_prefix(ftd::interpreter2::utils::CLONE))
            .unwrap_or(name);

        if self.get_initial_thing(name, line_number).is_ok() {
            return Ok(());
        }

        let name = self.resolve_name(name);

        let state = if let Some(state) = {
            match &mut self.bag {
                BagOrState::Bag(_) => None,
                BagOrState::State(s) => Some(s),
            }
        } {
            state
        } else {
            return self.err("not found", name, "search_thing", line_number);
        };

        let (doc_name, thing_name, _remaining) = // Todo: use remaining
            ftd::interpreter2::utils::get_doc_name_and_thing_name_and_remaining(
                name.as_str(),
                self.name,
                line_number,
            );

        if doc_name.eq(ftd::interpreter2::FTD_INHERITED) {
            return Ok(());
        }

        // let current_parsed_document = state.parsed_libs.get(self.name).unwrap();

        /*if doc_name.ne(self.name) {
            let current_doc_contains_thing = current_parsed_document
                .ast
                .iter()
                .filter(|v| {
                    !v.is_component()
                        && (v.name().eq(&format!("{}.{}", doc_name, thing_name))
                            || v.name()
                                .starts_with(format!("{}.{}.", doc_name, thing_name).as_str()))
                })
                .map(|v| (0, v.to_owned()))
                .collect_vec();
            if !current_doc_contains_thing.is_empty()
                && !state.to_process.contains.contains(&(
                    self.name.to_string(),
                    format!("{}#{}", doc_name, thing_name),
                ))
            {
                state
                    .to_process
                    .stack
                    .push((self.name.to_string(), current_doc_contains_thing));
                state.to_process.contains.insert((
                    self.name.to_string(),
                    format!("{}#{}", doc_name, thing_name),
                ));
            }
        }*/

        if let Some(parsed_document) = state.parsed_libs.get(doc_name.as_str()) {
            let ast_for_thing = parsed_document
                .ast
                .iter()
                .filter(|v| {
                    !v.is_component()
                        && (v.name().eq(&thing_name)
                            || v.name().starts_with(format!("{}.", thing_name).as_str()))
                })
                .map(|v| (0, v.to_owned()))
                .collect_vec();

            if ast_for_thing.is_empty() {
                if parsed_document
                    .foreign_variable
                    .iter()
                    .any(|v| thing_name.eq(v))
                {
                    state.pending_imports.stack.push((
                        doc_name.to_string(),
                        name,
                        line_number,
                        self.name.to_string(),
                    ));
                    state
                        .pending_imports
                        .contains
                        .insert((doc_name.to_string(), format!("{}#{}", doc_name, thing_name)));
                }

                return Ok(());
            }

            if !state
                .pending_imports
                .contains
                .contains(&(doc_name.to_string(), format!("{}#{}", doc_name, thing_name)))
            {
                state
                    .pending_imports
                    .contains
                    .insert((doc_name.to_string(), format!("{}#{}", doc_name, thing_name)));

                state.pending_imports.stack.push((
                    doc_name.to_string(),
                    name,
                    line_number,
                    self.name.to_string(),
                ));
            }
        } else {
            state.pending_imports.stack.push((
                doc_name.to_string(),
                name,
                line_number,
                self.name.to_string(),
            ));
            state
                .pending_imports
                .contains
                .insert((doc_name.to_string(), format!("{}#{}", doc_name, thing_name)));
        }

        Ok(())
    }

    pub fn search_thing(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::StateWithThing<ftd::interpreter2::Thing>>
    {
        let name = name
            .strip_prefix(ftd::interpreter2::utils::REFERENCE)
            .or_else(|| name.strip_prefix(ftd::interpreter2::utils::CLONE))
            .unwrap_or(name);

        let (initial_thing, remaining) =
            try_ok_state!(self.search_initial_thing(name, line_number)?);

        if let Some(remaining) = remaining {
            return search_thing_(self, line_number, remaining.as_str(), initial_thing);
        }
        return Ok(ftd::interpreter2::StateWithThing::new_thing(initial_thing));

        fn search_thing_(
            doc: &mut ftd::interpreter2::TDoc,
            line_number: usize,
            name: &str,
            thing: ftd::interpreter2::Thing,
        ) -> ftd::interpreter2::Result<ftd::interpreter2::StateWithThing<ftd::interpreter2::Thing>>
        {
            let (v, remaining) = ftd::interpreter2::utils::split_at(name, ".");
            let thing = match thing.clone() {
                ftd::interpreter2::Thing::Variable(ftd::interpreter2::Variable {
                    name,
                    value,
                    mutable,
                    ..
                }) => {
                    let value_kind = value.kind();
                    let fields = match value.resolve(doc, line_number)?.inner() {
                        Some(ftd::interpreter2::Value::Record { fields, .. }) => fields,
                        Some(ftd::interpreter2::Value::Object { values }) => values,
                        Some(ftd::interpreter2::Value::List { data, .. }) => data
                            .into_iter()
                            .enumerate()
                            .map(|(index, v)| (index.to_string(), v))
                            .collect::<ftd::Map<ftd::interpreter2::PropertyValue>>(),
                        None => {
                            let kind_name = match value_kind.get_record_name() {
                                Some(name) => name,
                                _ => {
                                    return doc.err(
                                        "not an record",
                                        thing,
                                        "search_thing_",
                                        line_number,
                                    );
                                }
                            };
                            let kind_thing =
                                try_ok_state!(doc.search_thing(kind_name, line_number)?);
                            let kind = match kind_thing
                                .record(doc.name, line_number)?
                                .fields
                                .iter()
                                .find(|f| f.name.eq(&v))
                                .map(|v| v.kind.to_owned())
                            {
                                Some(f) => f,
                                _ => {
                                    return doc.err(
                                        "not an record or or-type",
                                        thing,
                                        "search_thing_",
                                        line_number,
                                    );
                                }
                            };
                            let thing =
                                ftd::interpreter2::Thing::Variable(ftd::interpreter2::Variable {
                                    name,
                                    kind: kind.to_owned(),
                                    mutable,
                                    value: ftd::interpreter2::PropertyValue::Value {
                                        value: ftd::interpreter2::Value::Optional {
                                            data: Box::new(None),
                                            kind,
                                        },
                                        is_mutable: mutable,
                                        line_number,
                                    },
                                    conditional_value: vec![],
                                    line_number,
                                    is_static: !mutable,
                                });
                            if let Some(remaining) = remaining {
                                return search_thing_(doc, line_number, &remaining, thing);
                            }
                            return Ok(ftd::interpreter2::StateWithThing::new_thing(thing));
                        }
                        _ => return doc.err("not an record", thing, "search_thing_", line_number),
                    };
                    match fields.get(&v) {
                        Some(ftd::interpreter2::PropertyValue::Value {
                            value: val,
                            line_number,
                            is_mutable,
                        }) => ftd::interpreter2::Thing::Variable(ftd::interpreter2::Variable {
                            name,
                            kind: ftd::interpreter2::KindData {
                                kind: val.kind(),
                                caption: false,
                                body: false,
                            },
                            mutable: false,
                            value: ftd::interpreter2::PropertyValue::Value {
                                value: val.to_owned(),
                                line_number: *line_number,
                                is_mutable: *is_mutable,
                            },
                            conditional_value: vec![],
                            line_number: *line_number,
                            is_static: !mutable,
                        }),
                        Some(ftd::interpreter2::PropertyValue::Reference { name, .. })
                        | Some(ftd::interpreter2::PropertyValue::Clone { name, .. }) => {
                            let (initial_thing, name) =
                                try_ok_state!(doc.search_initial_thing(name, line_number)?);

                            if let Some(remaining) = name {
                                try_ok_state!(search_thing_(
                                    doc,
                                    line_number,
                                    remaining.as_str(),
                                    initial_thing,
                                )?)
                            } else {
                                initial_thing
                            }
                        }
                        _ => thing,
                    }
                }
                ftd::interpreter2::Thing::OrType(ftd::interpreter2::OrType {
                    name: or_type_name,
                    variants,
                    ..
                }) => {
                    if let Some(thing) = variants.into_iter().find(|or_type_variant| {
                        or_type_variant
                            .name()
                            .trim_start_matches(format!("{}.", or_type_name).as_str())
                            .eq(&v)
                    }) {
                        // Todo: Handle remaining
                        ftd::interpreter2::Thing::OrTypeWithVariant {
                            or_type: or_type_name.to_string(),
                            variant: thing,
                        }
                    } else {
                        return doc.err(
                            format!(
                                "Can't find variant `{}` in or-type `{}`",
                                name, or_type_name
                            )
                            .as_str(),
                            thing,
                            "search_thing_",
                            line_number,
                        );
                    }
                }
                _ => {
                    return doc.err(
                        format!("not an or-type `{}`", name).as_str(),
                        thing,
                        "search_thing_",
                        line_number,
                    );
                }
            };
            if let Some(remaining) = remaining {
                return search_thing_(doc, line_number, &remaining, thing);
            }
            Ok(ftd::interpreter2::StateWithThing::new_thing(thing))
        }
    }

    pub fn search_initial_thing(
        &mut self,
        name: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<
        ftd::interpreter2::StateWithThing<(ftd::interpreter2::Thing, Option<String>)>,
    > {
        use itertools::Itertools;

        let name = name
            .strip_prefix(ftd::interpreter2::utils::REFERENCE)
            .or_else(|| name.strip_prefix(ftd::interpreter2::utils::CLONE))
            .unwrap_or(name);

        if let Ok(thing) = self.get_initial_thing(name, line_number) {
            return Ok(ftd::interpreter2::StateWithThing::new_thing(thing));
        }

        let name = self.resolve_name(name);

        let state = if let Some(state) = {
            match &mut self.bag {
                BagOrState::Bag(_) => None,
                BagOrState::State(s) => Some(s),
            }
        } {
            state
        } else {
            return self.err("not found", name, "search_thing", line_number);
        };

        let (doc_name, thing_name, remaining) = // Todo: use remaining
            ftd::interpreter2::utils::get_doc_name_and_thing_name_and_remaining(
                name.as_str(),
                self.name,
                line_number,
            );

        let current_parsed_document = state.parsed_libs.get(state.id.as_str()).unwrap();

        if doc_name.ne(state.id.as_str()) {
            let current_doc_contains_thing = current_parsed_document
                .ast
                .iter()
                .filter(|v| {
                    let name = ftd::interpreter2::utils::resolve_name(
                        v.name().as_str(),
                        state.id.as_str(),
                        &current_parsed_document.doc_aliases,
                    );
                    !v.is_component()
                        && (name.eq(&format!("{}#{}", doc_name, thing_name))
                            || name.starts_with(format!("{}#{}.", doc_name, thing_name).as_str()))
                })
                .map(|v| (0, v.to_owned()))
                .collect_vec();
            if !current_doc_contains_thing.is_empty() {
                state
                    .to_process
                    .stack
                    .push((state.id.to_string(), current_doc_contains_thing));

                if !state
                    .to_process
                    .contains
                    .contains(&(state.id.to_string(), format!("{}#{}", doc_name, thing_name)))
                {
                    state
                        .to_process
                        .contains
                        .insert((state.id.to_string(), format!("{}#{}", doc_name, thing_name)));
                }
            } else if !current_doc_contains_thing.is_empty() && state.peek_stack().unwrap().1.gt(&4)
            {
                return self.err("not found", name, "search_thing", line_number);
            }
        }

        if let Some(parsed_document) = state.parsed_libs.get(doc_name.as_str()) {
            let ast_for_thing = parsed_document
                .ast
                .iter()
                .filter(|v| {
                    !v.is_component()
                        && (v.name().eq(&thing_name)
                            || v.name().starts_with(format!("{}.", thing_name).as_str()))
                })
                .map(|v| (0, v.to_owned()))
                .collect_vec();

            if ast_for_thing.is_empty() {
                if parsed_document
                    .foreign_variable
                    .iter()
                    .any(|v| thing_name.eq(v))
                {
                    return Ok(ftd::interpreter2::StateWithThing::new_state(
                        ftd::interpreter2::InterpreterWithoutState::StuckOnForeignVariable {
                            module: doc_name,
                            variable: remaining
                                .map(|v| format!("{}.{}", thing_name, v))
                                .unwrap_or(thing_name),
                            caller_module: self.name.to_string(),
                        },
                    ));
                }

                return self.err("not found", name, "search_thing", line_number);
            }

            state
                .to_process
                .stack
                .push((doc_name.to_string(), ast_for_thing));
            if !state
                .to_process
                .contains
                .contains(&(doc_name.to_string(), format!("{}#{}", doc_name, thing_name)))
            {
                state
                    .to_process
                    .contains
                    .insert((doc_name.to_string(), format!("{}#{}", doc_name, thing_name)));
            }

            return Ok(ftd::interpreter2::StateWithThing::new_continue());
        }

        if doc_name.eq(self.name) {
            return self.err("not found", name, "search_thing", line_number);
        }

        state.pending_imports.stack.push((
            doc_name.to_string(),
            name,
            line_number,
            self.name.to_string(),
        ));

        Ok(ftd::interpreter2::StateWithThing::new_state(
            ftd::interpreter2::InterpreterWithoutState::StuckOnImport {
                module: doc_name,
                caller_module: self.name.to_string(),
            },
        ))
    }

    pub fn get_initial_thing(
        &'a self,
        name: &'a str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<(ftd::interpreter2::Thing, Option<String>)> {
        let name = name
            .strip_prefix(ftd::interpreter2::utils::REFERENCE)
            .or_else(|| name.strip_prefix(ftd::interpreter2::utils::CLONE))
            .unwrap_or(name);

        let name = self.resolve_name(name);

        let (splited_name, remaining_value) = if let Ok(function_name) =
            ftd::interpreter2::utils::get_function_name(name.as_str(), self.name, line_number)
        {
            (function_name, None)
        } else {
            ftd::interpreter2::utils::get_doc_name_and_remaining(
                name.as_str(),
                self.name,
                line_number,
            )
        };

        match self.bag().get(splited_name.as_str()).map(ToOwned::to_owned) {
            Some(a) => Ok((a, remaining_value)),
            None => match self.bag().get(name.as_str()).map(|v| (v.to_owned(), None)) {
                Some(a) => Ok(a),
                None => self.err("not found", splited_name, "get_initial_thing", line_number),
            },
        }
    }

    pub fn from_json_rows(
        &self,
        rows: &[Vec<serde_json::Value>],
        kind: &ftd::interpreter2::Kind,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Value> {
        Ok(match kind {
            ftd::interpreter2::Kind::List { kind, .. } => {
                let mut data = vec![];
                for row in rows {
                    data.push(
                        self.from_json_row(row, kind.as_ref(), line_number)?
                            .into_property_value(false, line_number),
                    );
                }

                ftd::interpreter2::Value::List {
                    data,
                    kind: kind.to_owned().into_kind_data(),
                }
            }
            t => unimplemented!(
                "{:?} not yet implemented, line number: {}, doc: {}",
                t,
                line_number,
                self.name.to_string()
            ),
        })
    }

    pub fn from_json_row(
        &self,
        row: &[serde_json::Value],
        kind: &ftd::interpreter2::Kind,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Value> {
        Ok(match kind {
            ftd::interpreter2::Kind::Record { name, .. } => {
                let rec = self.get_record(name, line_number)?;
                let rec_fields = rec.fields;
                let mut fields: ftd::Map<ftd::interpreter2::PropertyValue> = Default::default();
                for (idx, key) in rec_fields.iter().enumerate() {
                    let val = match row.get(idx) {
                        Some(v) => v,
                        None => {
                            return ftd::interpreter2::utils::e2(
                                format!("key not found: {}", key.name.as_str()),
                                self.name,
                                line_number,
                            )
                        }
                    };
                    fields.insert(
                        key.name.to_string(),
                        self.from_json(val, &key.kind.kind, line_number)?
                            .into_property_value(false, line_number),
                    );
                }
                ftd::interpreter2::Value::Record {
                    name: name.to_string(),
                    fields,
                }
            }
            ftd::interpreter2::Kind::String { .. } if row.first().is_some() => {
                ftd::interpreter2::Value::String {
                    text: serde_json::from_value::<String>(row.first().unwrap().to_owned())
                        .map_err(|_| ftd::interpreter2::Error::ParseError {
                            message: format!("Can't parse to string, found: {:?}", row),
                            doc_id: self.name.to_string(),
                            line_number,
                        })?,
                }
            }
            ftd::interpreter2::Kind::Integer { .. } if row.first().is_some() => {
                ftd::interpreter2::Value::Integer {
                    value: serde_json::from_value::<i64>(row.first().unwrap().to_owned()).map_err(
                        |_| ftd::interpreter2::Error::ParseError {
                            message: format!("Can't parse to integer, found: {:?}", row),
                            doc_id: self.name.to_string(),
                            line_number,
                        },
                    )?,
                }
            }
            ftd::interpreter2::Kind::Decimal { .. } if row.first().is_some() => {
                ftd::interpreter2::Value::Decimal {
                    value: serde_json::from_value::<f64>(row.first().unwrap().to_owned()).map_err(
                        |_| ftd::interpreter2::Error::ParseError {
                            message: format!("Can't parse to decimal, found: {:?}", row),
                            doc_id: self.name.to_string(),
                            line_number,
                        },
                    )?,
                }
            }
            ftd::interpreter2::Kind::Boolean { .. } if row.first().is_some() => {
                ftd::interpreter2::Value::Boolean {
                    value: serde_json::from_value::<bool>(row.first().unwrap().to_owned())
                        .map_err(|_| ftd::interpreter2::Error::ParseError {
                            message: format!("Can't parse to boolean,found: {:?}", row),
                            doc_id: self.name.to_string(),
                            line_number,
                        })?,
                }
            }
            t => unimplemented!(
                "{:?} not yet implemented, line number: {}, doc: {}",
                t,
                line_number,
                self.name.to_string()
            ),
        })
    }

    pub fn from_json<T>(
        &self,
        json: &T,
        kind: &ftd::interpreter2::Kind,
        line_number: usize,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Value>
    where
        T: serde::Serialize + std::fmt::Debug,
    {
        let json =
            serde_json::to_value(json).map_err(|e| ftd::interpreter2::Error::ParseError {
                message: format!("Can't serialize to json: {:?}, found: {:?}", e, json),
                doc_id: self.name.to_string(),
                line_number,
            })?;

        self.as_json_(line_number, &json, kind.to_owned())
    }

    fn as_json_(
        &self,
        line_number: usize,
        json: &serde_json::Value,
        kind: ftd::interpreter2::Kind,
    ) -> ftd::interpreter2::Result<ftd::interpreter2::Value> {
        Ok(match kind {
            ftd::interpreter2::Kind::String { .. } => ftd::interpreter2::Value::String {
                text: serde_json::from_value::<String>(json.to_owned()).map_err(|_| {
                    ftd::interpreter2::Error::ParseError {
                        message: format!("Can't parse to string, found: {}", json),
                        doc_id: self.name.to_string(),
                        line_number,
                    }
                })?,
            },
            ftd::interpreter2::Kind::Integer { .. } => ftd::interpreter2::Value::Integer {
                value: serde_json::from_value::<i64>(json.to_owned()).map_err(|_| {
                    ftd::interpreter2::Error::ParseError {
                        message: format!("Can't parse to integer, found: {}", json),
                        doc_id: self.name.to_string(),
                        line_number,
                    }
                })?,
            },
            ftd::interpreter2::Kind::Decimal { .. } => ftd::interpreter2::Value::Decimal {
                value: serde_json::from_value::<f64>(json.to_owned()).map_err(|_| {
                    ftd::interpreter2::Error::ParseError {
                        message: format!("Can't parse to decimal, found: {}", json),
                        doc_id: self.name.to_string(),
                        line_number,
                    }
                })?,
            },
            ftd::interpreter2::Kind::Boolean { .. } => ftd::interpreter2::Value::Boolean {
                value: serde_json::from_value::<bool>(json.to_owned()).map_err(|_| {
                    ftd::interpreter2::Error::ParseError {
                        message: format!("Can't parse to boolean,found: {}", json),
                        doc_id: self.name.to_string(),
                        line_number,
                    }
                })?,
            },
            ftd::interpreter2::Kind::Record { name, .. } => {
                let rec_fields = self.get_record(&name, line_number)?.fields;
                let mut fields: ftd::Map<ftd::interpreter2::PropertyValue> = Default::default();
                if let serde_json::Value::Object(o) = json {
                    for field in rec_fields {
                        let val = match o.get(&field.name) {
                            Some(v) => v,
                            None => {
                                return ftd::interpreter2::utils::e2(
                                    format!("key not found: {}", field.name.as_str()),
                                    self.name,
                                    line_number,
                                )
                            }
                        };
                        fields.insert(
                            field.name,
                            ftd::interpreter2::PropertyValue::Value {
                                value: self.as_json_(line_number, val, field.kind.kind)?,
                                is_mutable: false,
                                line_number,
                            },
                        );
                    }
                } else {
                    return ftd::interpreter2::utils::e2(
                        format!("expected object of record type, found: {}", json),
                        self.name,
                        line_number,
                    );
                }
                ftd::interpreter2::Value::Record { name, fields }
            }
            ftd::interpreter2::Kind::List { kind, .. } => {
                let kind = kind.as_ref();
                let mut data: Vec<ftd::interpreter2::PropertyValue> = vec![];
                if let serde_json::Value::Array(list) = json {
                    for item in list {
                        data.push(ftd::interpreter2::PropertyValue::Value {
                            value: self.as_json_(line_number, item, kind.to_owned())?,
                            is_mutable: false,
                            line_number,
                        });
                    }
                } else {
                    return ftd::interpreter2::utils::e2(
                        format!("expected object of list type, found: {}", json),
                        self.name,
                        line_number,
                    );
                }
                ftd::interpreter2::Value::List {
                    data,
                    kind: kind.to_owned().into_kind_data(),
                }
            }
            ftd::interpreter2::Kind::Optional { kind, .. } => {
                let kind = kind.as_ref().to_owned();
                match json {
                    serde_json::Value::Null => ftd::interpreter2::Value::Optional {
                        kind: kind.into_kind_data(),
                        data: Box::new(None),
                    },
                    _ => self.as_json_(line_number, json, kind)?,
                }
            }
            t => unimplemented!(
                "{:?} not yet implemented, line number: {}, doc: {}",
                t,
                line_number,
                self.name.to_string()
            ),
        })
    }

    pub(crate) fn err<T, T2: std::fmt::Debug>(
        &self,
        msg: &str,
        ctx: T2,
        f: &str,
        line_number: usize,
    ) -> ftd::interpreter2::Result<T> {
        ftd::interpreter2::utils::e2(
            format!("{}: {} ({:?}), f: {}", self.name, msg, ctx, f),
            self.name,
            line_number,
        )
    }
}
