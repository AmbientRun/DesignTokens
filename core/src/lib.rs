use std::collections::HashMap;

use convert_case::{Case, Casing};
use expression::{Expression, Value};
use extensions::Extensions;
use indexmap::IndexMap;
use itertools::Itertools;
use serde::Deserialize;
mod expression;
pub mod extensions;

pub fn get_design_tokens() -> DesignTokens {
    let data = include_str!("../../ambient-design/tokens.json");
    serde_json::from_str(data).unwrap()
}

#[derive(Debug, Deserialize)]
pub struct DesignTokens {
    pub global: TokenOrGroup,
}
impl DesignTokens {
    pub fn to_css(&self) -> String {
        self.global.to_css(self, "")
    }
    pub fn to_rust(&self) -> String {
        self.global.to_rust(self, "")
    }
    fn get_value(&self, path: &[String]) -> &TokenValue {
        self.global.get_value(self, path)
    }
}

#[derive(Debug, Deserialize)]
pub enum TokenType {
    #[serde(rename = "border")]
    Border,
    #[serde(rename = "typography")]
    Typography,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenOrGroup {
    Token {
        value: TokenValue,
        #[serde(rename = "type")]
        type_: TokenType,
        #[serde(rename = "$extensions")]
        extensions: Option<Extensions>,
    },
    Group(IndexMap<String, TokenOrGroup>),
}
impl TokenOrGroup {
    fn to_css(&self, tokens: &DesignTokens, path: &str) -> String {
        match self {
            TokenOrGroup::Token {
                value,
                type_,
                extensions,
            } => match value {
                TokenValue::Single(value) => {
                    let value = match extensions {
                        Some(Extensions::StudioTokens(ext)) => ext.to_css(&value.get_value(tokens)),
                        None => value.to_css(tokens),
                    };
                    format!(":root {{ {path}: {}; }}", value)
                }
                TokenValue::Dict(dict) => {
                    let value = dict
                        .iter()
                        .map(|(key, value)| css_entry(tokens, type_, key, value))
                        .join("\n");
                    format!(".{path} {{\n{}\n}}", value)
                }
            },
            TokenOrGroup::Group(group) => group
                .iter()
                .map(|(key, value)| value.to_css(tokens, &format!("{path}--{}", slugify_css(key))))
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
                        None => value.get_value(tokens),
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
    fn get_value(&self, tokens: &DesignTokens, path: &[String]) -> &TokenValue {
        match self {
            TokenOrGroup::Token { value, .. } => {
                assert_eq!(path.len(), 0);
                value
            }
            TokenOrGroup::Group(group) => {
                group.get(&path[0]).unwrap().get_value(tokens, &path[1..])
            }
        }
    }
}
fn css_entry(tokens: &DesignTokens, type_: &TokenType, key: &str, value: &Expression) -> String {
    format!("{}: {};", css_property(type_, key), value.to_css(tokens))
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
            "textCase" => "text-transform".to_string(),
            _ => key.to_case(Case::Kebab),
        },
        _ => key.to_case(Case::Kebab),
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
    println!("{}", tokens.to_css());
    println!("{}", tokens.to_rust());
}
