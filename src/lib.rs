#![allow(clippy::derive_partial_eq_without_eq, clippy::get_first)]

extern crate self as ftd;

#[cfg(test)]
#[macro_use]
pub(crate) mod test;

pub mod ast;
pub mod code;
mod component;
mod condition;
mod constants;
mod di;
mod dnode;
pub mod evalexpr;
mod event;
mod execute_doc;
pub mod executor;
mod html;
pub mod html1;
pub mod interpreter;
pub mod interpreter2;
pub mod markup;
pub mod node;
mod or_type;
pub mod p1;
pub mod p11;
pub mod p2;
pub(crate) mod rendered;
mod rt;
mod ui;
mod value_with_default;
pub(crate) mod variable;
mod youtube_id;

pub use component::{ChildComponent, Component, Instruction};
pub use condition::Condition;
pub use constants::{identifier, regex};
pub use event::{Action, Event};
pub use ftd::{
    ftd::p2::interpreter::{interpret, Interpreter, InterpreterState, ParsedDocument},
    value_with_default::ValueWithDefault,
};
pub use html::{anchor, color, length, overflow, Collector, Node, StyleSpec};
pub use or_type::OrType;
pub use rendered::Rendered;
pub use rt::RT;
pub use ui::{
    Anchor, AttributeType, Code, Color, ColorValue, Column, Common, ConditionalAttribute,
    ConditionalValue, Container, Element, FontDisplay, GradientDirection, Grid, IFrame, IText,
    Image, ImageSrc, Input, Length, Loading, Markup, Markups, NamedFont, Overflow, Position,
    Region, Row, Scene, Spacing, Style, Text, TextAlign, TextBlock, TextFormat, Type, Weight,
};
pub use variable::{PropertyValue, TextSource, Value, Variable, VariableFlags};

pub fn js() -> String {
    include_str!("../ftd.js").replace("if (true) { // false", "if (false) { // false")
}

pub fn css() -> &'static str {
    include_str!("../ftd.css")
}
pub fn html() -> &'static str {
    include_str!("../ftd.html")
}

pub fn build() -> &'static str {
    include_str!("../build.html")
}

pub fn build_js() -> &'static str {
    include_str!("../build.js")
}

// #[cfg(test)]
pub type Map<T> = std::collections::BTreeMap<String, T>;

#[derive(serde::Deserialize, Debug, PartialEq, Default, Clone, serde::Serialize)]
pub struct VecMap<T> {
    value: Map<Vec<T>>,
}

impl<T: std::cmp::PartialEq> VecMap<T> {
    pub fn insert(&mut self, key: String, value: T) {
        if let Some(v) = self.value.get_mut(&key) {
            v.push(value);
        } else {
            self.value.insert(key, vec![value]);
        }
    }

    pub fn unique_insert(&mut self, key: String, value: T) {
        if let Some(v) = self.value.get_mut(&key) {
            if !v.contains(&value) {
                v.push(value);
            }
        } else {
            self.value.insert(key, vec![value]);
        }
    }

    pub fn extend(&mut self, key: String, value: Vec<T>) {
        if let Some(v) = self.value.get_mut(&key) {
            v.extend(value);
        } else {
            self.value.insert(key, value);
        }
    }

    pub fn get_value(&self, key: &str) -> Vec<&T> {
        let mut values = vec![];
        if let Some(v) = self.value.iter().find_map(|(k, v)| {
            if k.eq(key) || k.starts_with(format!("{}.", key).as_str()) {
                Some(v)
            } else {
                None
            }
        }) {
            values.extend(v)
        }
        values
    }
}

// #[cfg(not(test))]
// pub type Map<T> = std::collections::HashMap<String, T>;

#[derive(serde::Deserialize, Debug, PartialEq, Clone, serde::Serialize, Default)]
pub struct Document {
    pub html: String,
    pub data: ftd::DataDependenciesMap,
    pub external_children: ExternalChildrenDependenciesMap,
    pub body_events: String,
    pub css_collector: String,
}

// Condensed form of page-heading item stored by parsed document
#[derive(Debug, Clone, serde::Serialize)]
pub struct PageHeadingItem {
    pub url: Option<String>,
    pub title: Option<String>,
    pub region: Option<ftd::Region>,
    pub number: Option<String>,
    pub children: Vec<PageHeadingItem>,
}

// Page-heading struct identical with fpm::library::toc::TocItemCompat
// to be used by page-headings processor
#[derive(Debug, Clone, serde::Serialize)]
pub struct PageHeadingItemCompat {
    pub url: Option<String>,
    pub number: Option<String>,
    pub title: Option<String>,
    pub path: Option<String>,
    #[serde(rename = "is-heading")]
    pub is_heading: bool,
    // TODO: Font icon mapping to html?
    #[serde(rename = "font-icon")]
    pub font_icon: Option<String>,
    #[serde(rename = "is-disabled")]
    pub is_disabled: bool,
    #[serde(rename = "is-active")]
    pub is_active: bool,
    #[serde(rename = "is-open")]
    pub is_open: bool,
    #[serde(rename = "img-src")]
    pub image_src: Option<String>,
    pub document: Option<String>,
    pub children: Vec<PageHeadingItemCompat>,
}

// TextSource location = (is_from_section = T/F, subsection_index if is_from_section = F else 0)
pub type TextSourceLocation = (bool, usize);
pub type TextSourceWithLocation = (ftd::TextSource, TextSourceLocation);

// ReplaceLinkBlock = (Id, TextSourceWithLocation, Line number)
// contains relevant id data associated with links along with its source
// from where those were captured and where link replacement or escaped links
// needs to be resolved
pub type ReplaceLinkBlock<T> = (T, ftd::TextSourceWithLocation, usize);

pub type DataDependenciesMap = ftd::Map<Data>;

#[derive(serde::Deserialize, Debug, PartialEq, Clone, serde::Serialize, Default)]
pub struct Data {
    pub value: serde_json::Value,
    pub dependencies: ftd::Map<serde_json::Value>,
}

pub type ExternalChildrenDependenciesMap = ftd::Map<Vec<ExternalChildrenCondition>>;

#[derive(serde::Deserialize, Debug, PartialEq, Clone, serde::Serialize, Default)]
pub struct ExternalChildrenCondition {
    pub condition: Vec<String>,
    pub set_at: String,
}

#[derive(serde::Deserialize, Debug, PartialEq, Clone, serde::Serialize)]
pub enum DependencyType {
    Style,
    Visible,
    Value,
    Variable,
}

#[derive(serde::Deserialize, Debug, PartialEq, Clone, serde::Serialize)]
pub struct Dependencies {
    pub dependency_type: DependencyType,
    pub condition: Option<serde_json::Value>,
    pub parameters: ftd::Map<ConditionalValueWithDefault>,
    pub remaining: Option<String>,
}

#[derive(serde::Deserialize, Debug, PartialEq, Clone, serde::Serialize, Default)]
pub struct ConditionalValueWithDefault {
    pub value: ConditionalValue,
    pub default: Option<ConditionalValue>,
}

pub struct ExampleLibrary {}

impl ExampleLibrary {
    pub fn dummy_global_ids_map(&self) -> std::collections::HashMap<String, String> {
        let mut global_ids: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();

        global_ids.insert("foo".to_string(), "/foo/bar/#foo".to_string());
        global_ids.insert("hello".to_string(), "/hello/there/#hello".to_string());
        global_ids.insert("some id".to_string(), "/some/id/#some-id".to_string());

        // To debug for section
        global_ids.insert("scp".to_string(), "/foo/bar/#scp".to_string());
        global_ids.insert("sh".to_string(), "/hello/there/#sh".to_string());
        global_ids.insert("sb".to_string(), "/some/id/#sb".to_string());

        // To debug for subsection
        global_ids.insert("sscp".to_string(), "/foo/bar/#sscp".to_string());
        global_ids.insert("ssh".to_string(), "/hello/there/#ssh".to_string());
        global_ids.insert("ssb".to_string(), "/some/id/#ssb".to_string());

        // More dummy instances for debugging purposes
        global_ids.insert("a".to_string(), "/some/#a".to_string());
        global_ids.insert("b".to_string(), "/some/#b".to_string());
        global_ids.insert("c".to_string(), "/some/#c".to_string());
        global_ids.insert("d".to_string(), "/some/#d".to_string());

        // to debug in case of checkboxes
        global_ids.insert("x".to_string(), "/some/#x".to_string());
        global_ids.insert("X".to_string(), "/some/#X".to_string());

        global_ids
    }

    pub fn get(&self, name: &str, _doc: &ftd::p2::TDoc) -> Option<String> {
        std::fs::read_to_string(format!("./examples/{}.ftd", name)).ok()
    }

    /// checks if the current processor is a lazy processor
    /// or not
    ///
    /// for more details
    /// visit www.fpm.dev/glossary/#lazy-processor
    pub fn is_lazy_processor(
        section: &ftd::p1::Section,
        doc: &ftd::p2::TDoc,
    ) -> ftd::p1::Result<bool> {
        Ok(section
            .header
            .str(doc.name, section.line_number, "$processor$")?
            .eq("page-headings"))
    }

    pub fn process(
        &self,
        section: &ftd::p1::Section,
        doc: &ftd::p2::TDoc,
    ) -> ftd::p1::Result<ftd::Value> {
        ftd::p2::utils::unknown_processor_error(
            format!("unimplemented for section {:?} and doc {:?}", section, doc),
            doc.name.to_string(),
            section.line_number,
        )
    }

    pub fn get_with_result(&self, name: &str, doc: &ftd::p2::TDoc) -> ftd::p1::Result<String> {
        match self.get(name, doc) {
            Some(v) => Ok(v),
            None => ftd::p2::utils::e2(format!("library not found: {}", name), "", 0),
        }
    }
}
