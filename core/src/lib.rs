use std::collections::HashMap;

use convert_case::{Case, Casing};
use expression::{Expression, NumberType, Value};
use extensions::Extensions;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::Deserialize;
mod expression;
pub mod extensions;

pub fn get_design_tokens() -> Vec<DesignTokens> {
    // I couldn't get one exporter to give me good, well-formatted data, so I had to use two.
    let mut data: Vec<DesignTokens> =
        serde_json::from_str(include_str!("./exportedVariables.json")).unwrap();
    let data2: IndexMap<String, TokenOrGroup> =
        serde_json::from_str(include_str!("./design-tokens.tokens.json")).unwrap();
    data.push(DesignTokens {
        file_name: None,
        body: TokenOrGroup::Group(data2),
    });
    data
}

#[derive(Debug, Deserialize)]
pub struct DesignTokens {
    #[serde(rename = "fileName")]
    pub file_name: Option<String>,
    pub body: TokenOrGroup,
}
impl DesignTokens {
    pub fn get_name(&self) -> &str {
        if let Some(name) = &self.file_name {
            name.split(".").nth(1).unwrap()
        } else {
            "ambient"
        }
    }
    pub fn get_name_rust(&self) -> String {
        slugify_rs(self.get_name()).to_case(Case::UpperFlat)
    }
    pub fn to_css(&self) -> String {
        self.body.to_css(self, &slugify_css(self.get_name()), "")
    }
    pub fn to_rust(&self) -> String {
        self.body.to_rust(self, "")
    }
    fn get_value(&self, path: &[String]) -> Option<&TokenValue> {
        self.body.get_value(self, path)
    }
}

#[derive(Debug, Deserialize, Default)]
pub enum TokenType {
    #[default]
    None,
    #[serde(rename = "border")]
    Border,
    #[serde(rename = "typography", alias = "custom-fontStyle")]
    Typography,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenOrGroup {
    Token {
        #[serde(alias = "$value")]
        value: TokenValue,
        #[serde(rename = "type", alias = "$type")]
        type_: TokenType,
        #[serde(rename = "$extensions")]
        extensions: Option<Extensions>,
    },
    Group(IndexMap<String, TokenOrGroup>),
}
impl TokenOrGroup {
    fn to_css(&self, tokens: &DesignTokens, root_class: &str, path: &str) -> String {
        match self {
            TokenOrGroup::Token {
                value,
                type_,
                extensions,
            } => match value {
                TokenValue::Single(value) => {
                    let value = match extensions {
                        Some(Extensions::StudioTokens(ext)) => ext.to_css(&value.get_value(tokens)),
                        _ => css_value(tokens, value),
                    };
                    format!(".{root_class} {{ -{path}: {}; }}", value)
                }
                TokenValue::Dict(dict) => {
                    let value = dict
                        .iter()
                        .map(|(key, value)| css_entry(tokens, type_, key, value))
                        .join("\n");
                    format!(".{root_class} .{} {{\n{}\n}}", &path[1..], value)
                }
            },
            TokenOrGroup::Group(group) => group
                .iter()
                .map(|(key, value)| {
                    value.to_css(tokens, root_class, &format!("{path}-{}", slugify_css(key)))
                })
                .join("\n"),
        }
    }
    fn to_rust(&self, tokens: &DesignTokens, path: &str) -> String {
        match self {
            TokenOrGroup::Token {
                value, extensions, ..
            } => match value {
                TokenValue::Single(value) => {
                    let value = match extensions {
                        Some(Extensions::StudioTokens(ext)) => {
                            ext.to_rust(&value.get_value(tokens))
                        }
                        _ => value.get_value(tokens),
                    };
                    format!(
                        "pub const {path}: {} = {};",
                        value.to_rust_type(),
                        value.to_rust()
                    )
                }
                TokenValue::Dict(dict) => {
                    let value = dict
                        .iter()
                        .map(|(key, value)| {
                            format!(
                                "(\"{}\", {})",
                                key,
                                value.get_value(tokens).to_rust_string()
                            )
                        })
                        .join(", ");
                    format!(
                        "pub const {path}: &'static [(&'static str, &'static str)] = &[{}];",
                        value
                    )
                }
            },
            TokenOrGroup::Group(group) => group
                .iter()
                .map(|(key, value)| {
                    let key = slugify_rs(key).to_case(Case::UpperFlat);
                    value.to_rust(
                        tokens,
                        &if path.is_empty() {
                            key
                        } else {
                            format!("{path}_{}", key)
                        },
                    )
                })
                .join("\n"),
        }
    }
    fn get_value(&self, tokens: &DesignTokens, path: &[String]) -> Option<&TokenValue> {
        match self {
            TokenOrGroup::Token { value, .. } => {
                assert_eq!(path.len(), 0);
                Some(value)
            }
            TokenOrGroup::Group(group) => group.get(&path[0])?.get_value(tokens, &path[1..]),
        }
    }
}
fn css_entry(tokens: &DesignTokens, type_: &TokenType, key: &str, value: &Expression) -> String {
    let prop = css_property(type_, key);
    format!("{}: {};", prop, css_value(tokens, value))
}
fn css_property(type_: &TokenType, key: &str) -> String {
    match type_ {
        TokenType::Border => match key {
            "color" => "border-color".to_string(),
            "width" => "border-width".to_string(),
            "style" => "border-style".to_string(),
            _ => key.to_case(Case::Kebab),
        },
        TokenType::Typography => match key {
            "textCase" | "text-case" => "text-transform".to_string(),
            "lineHeight" => "line-height".to_string(),
            _ => key.to_case(Case::Kebab),
        },
        _ => key.to_case(Case::Kebab),
    }
}
fn css_value(tokens: &DesignTokens, value: &Expression) -> String {
    match value {
        Expression::Value(Value::Number(v, NumberType::None)) => {
            Expression::Value(Value::Number(*v, NumberType::Pixels)).to_css(tokens)
        }
        _ => value.to_css(tokens),
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenValue {
    Single(Expression),
    Dict(HashMap<String, Expression>),
}
impl TokenValue {
    fn get_value(&self, tokens: &DesignTokens) -> Value {
        match self {
            TokenValue::Single(expr) => expr.get_value(tokens),
            _ => panic!("Can't resolve"),
        }
    }
}
impl Default for TokenValue {
    fn default() -> Self {
        Self::Dict(Default::default())
    }
}

pub(crate) fn slugify(s: &str, sep: &str) -> String {
    // let chars = s.chars().map(|c| c.is_ascii_alphanumeric()).collect::<String>();
    deunicode::deunicode(
        &s.replace(',', "c")
            .replace('+', "p")
            .replace('.', "d")
            .replace('(', "_")
            .replace(')', "_")
            .replace(' ', sep)
            .to_ascii_lowercase(),
    )
}
pub(crate) fn slugify_rs(s: &str) -> String {
    slugify(s, "_")
}
pub(crate) fn slugify_css(s: &str) -> String {
    slugify(s, "-")
}

#[test]
fn test() {
    let tokens = get_design_tokens();
    for tokens in tokens {
        println!("{}", tokens.to_css());
        println!("{}", tokens.to_rust());
    }
}
