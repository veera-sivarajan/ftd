#[derive(Debug, PartialEq, Clone)]
pub struct Variable {
    pub name: String,
    pub value: ftd::interpreter::PropertyValue,
    pub conditions: Vec<ConditionalValue>,
    pub flags: VariableFlags,
}

impl Variable {
    pub(crate) fn from_p1_section(
        s: &ftd::p11::Section,
        doc_id: &str,
    ) -> ftd::interpreter::Result<Variable> {
        let value = ftd::interpreter::PropertyValue::from_p1_section(s, doc_id)?;
        if !s.headers.find("if").is_empty() {
            return Err(ftd::interpreter::Error::ParseError {
                message: format!(
                    "`if` can't be present in variable declaration for section: `{}`",
                    s.name
                ),
                doc_id: doc_id.to_string(),
                line_number: s.line_number,
            });
        }
        let flags = Variable::get_flags(s, doc_id)?;
        Ok(Variable {
            name: s.name.to_string(),
            value,
            conditions: vec![],
            flags,
        })
    }

    pub(crate) fn get_flags(
        s: &ftd::p11::Section,
        doc_id: &str,
    ) -> ftd::p11::Result<VariableFlags> {
        let header = match ftd::interpreter::PropertyValue::for_header_with_kind(
            s,
            doc_id,
            ALWAYS_INCLUDE,
            &ftd::interpreter::KindData::boolean(),
        ) {
            Ok(val) => val,
            _ => return Ok(VariableFlags::default()),
        };

        match header {
            ftd::interpreter::PropertyValue::Value {
                value: ftd::interpreter::Value::Boolean { value },
            } => Ok(VariableFlags {
                always_include: Some(value),
            }),
            ftd::interpreter::PropertyValue::Reference { .. } => unimplemented!(),
            t => Err(ftd::p11::Error::ParseError {
                message: format!("Expected boolean found: {:?}", t),
                doc_id: doc_id.to_string(),
                line_number: s.line_number,
            }),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ConditionalValue {
    pub expression: ftd::interpreter::Boolean,
    pub value: ftd::interpreter::PropertyValue,
}

#[derive(Debug, PartialEq, Clone, Default)]
pub struct VariableFlags {
    pub always_include: Option<bool>,
}

pub const ALWAYS_INCLUDE: &str = "$always-include$";

#[cfg(test)]
mod test {
    #[track_caller]
    fn p(s: &str, t: ftd::interpreter::Variable) {
        let section = ftd::p11::parse(s, "foo")
            .unwrap_or_else(|e| panic!("{:?}", e))
            .first()
            .unwrap()
            .to_owned();
        assert_eq!(
            super::Variable::from_p1_section(&section, "foo").unwrap_or_else(|e| panic!("{:?}", e)),
            t
        )
    }

    #[track_caller]
    fn f(s: &str, m: &str) {
        let section = ftd::p11::parse(s, "foo")
            .unwrap_or_else(|e| panic!("{:?}", e))
            .first()
            .unwrap()
            .to_owned();
        match super::Variable::from_p1_section(&section, "foo") {
            Ok(r) => panic!("expected failure, found: {:?}", r),
            Err(e) => {
                let expected = m.trim();
                let f2 = e.to_string();
                let found = f2.trim();
                if expected != found {
                    let patch = diffy::create_patch(expected, found);
                    let f = diffy::PatchFormatter::new().with_color();
                    print!(
                        "{}",
                        f.fmt_patch(&patch)
                            .to_string()
                            .replace("\\ No newline at end of file", "")
                    );
                    println!("expected:\n{}\nfound:\n{}\n", expected, f2);
                    panic!("test failed")
                }
            }
        }
    }

    #[test]
    fn integer() {
        p(
            "-- integer age: 40",
            super::Variable {
                name: "age".to_string(),
                value: ftd::interpreter::PropertyValue::Value {
                    value: ftd::interpreter::Value::Integer { value: 40 },
                },
                conditions: vec![],
                flags: Default::default(),
            },
        )
    }

    #[test]
    fn integer_list() {
        p(
            indoc::indoc!(
                "
            -- integer list ages: 
            
            -- integer: 40

            -- integer: 50

            -- end: ages
            "
            ),
            super::Variable {
                name: "ages".to_string(),
                value: ftd::interpreter::PropertyValue::Value {
                    value: ftd::interpreter::Value::List {
                        data: vec![
                            ftd::interpreter::PropertyValue::Value {
                                value: ftd::interpreter::Value::Integer { value: 40 },
                            },
                            ftd::interpreter::PropertyValue::Value {
                                value: ftd::interpreter::Value::Integer { value: 50 },
                            },
                        ],
                        kind: ftd::interpreter::KindData {
                            kind: ftd::interpreter::Kind::List {
                                kind: Box::new(ftd::interpreter::Kind::Integer),
                            },
                            caption: false,
                            body: false,
                        },
                    },
                },
                conditions: vec![],
                flags: Default::default(),
            },
        );

        f(indoc::indoc!(
            "
            -- integer list ages: 
            
            -- integer: 40

            -- string: 50

            -- end: ages
            "
            ),
          "InvalidKind: foo:5 -> List kind mismatch, expected kind `Integer`, found kind `String`"
        )
    }

    #[test]
    fn optional() {
        p(
            "-- optional integer age: 40",
            super::Variable {
                name: "age".to_string(),
                value: ftd::interpreter::PropertyValue::Value {
                    value: ftd::interpreter::Value::Optional {
                        data: Box::new(Some(ftd::interpreter::Value::Integer { value: 40 })),
                        kind: ftd::interpreter::KindData {
                            kind: ftd::interpreter::Kind::Integer,
                            caption: false,
                            body: false,
                        },
                    },
                },
                conditions: vec![],
                flags: Default::default(),
            },
        );

        p(
            "-- optional integer age: ",
            super::Variable {
                name: "age".to_string(),
                value: ftd::interpreter::PropertyValue::Value {
                    value: ftd::interpreter::Value::Optional {
                        data: Box::new(None),
                        kind: ftd::interpreter::KindData {
                            kind: ftd::interpreter::Kind::Integer,
                            caption: false,
                            body: false,
                        },
                    },
                },
                conditions: vec![],
                flags: Default::default(),
            },
        )
    }
}
