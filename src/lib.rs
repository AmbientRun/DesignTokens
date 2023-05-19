use std::collections::HashMap;

use expression::{Expression, Value};
use extensions::{Extensions, StudioTokensExtension};
use itertools::Itertools;
use serde::Deserialize;
use slug::slugify;
mod expression;
pub mod extensions;

#[derive(Debug, Deserialize)]
pub struct DesignTokens {
    pub global: TokenOrGroup,
}
impl DesignTokens {
    pub fn to_css(&self) -> String {
        let inner = self.global.to_css(self, "");
        format!(":root {{\n{inner}\n}}")
    }
    fn get_value(&self, path: &[String]) -> &TokenValue {
        self.global.get_value(self, path)
    }
}
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenOrGroup {
    Token {
        value: TokenValue,
        #[serde(rename = "type")]
        type_: String,
        #[serde(rename = "$extensions")]
        extensions: Option<Extensions>,
    },
    Group(HashMap<String, TokenOrGroup>),
}
impl TokenOrGroup {
    fn to_css(&self, tokens: &DesignTokens, path: &str) -> String {
        match self {
            TokenOrGroup::Token {
                value,
                type_,
                extensions,
            } => {
                let value = match extensions {
                    Some(Extensions::StudioTokens(ext)) => ext.to_css(&value.get_value(tokens)),
                    None => value.to_css(tokens),
                };
                format!("{path}: {};", value)
            }
            TokenOrGroup::Group(group) => group
                .iter()
                .map(|(key, value)| value.to_css(tokens, &format!("{path}--{}", slugify(key))))
                .join("\n"),
        }
    }
    fn get_value(&self, tokens: &DesignTokens, path: &[String]) -> &TokenValue {
        match self {
            TokenOrGroup::Token {
                value,
                type_,
                extensions,
            } => {
                assert_eq!(path.len(), 0);
                value
            }
            TokenOrGroup::Group(group) => {
                group.get(&path[0]).unwrap().get_value(tokens, &path[1..])
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TokenValue {
    String(Expression),
    Dict(HashMap<String, Expression>),
}
impl TokenValue {
    fn to_css(&self, tokens: &DesignTokens) -> String {
        match self {
            TokenValue::String(value) => value.to_css(tokens),
            TokenValue::Dict(_) => todo!(),
        }
    }
    fn get_value(&self, tokens: &DesignTokens) -> Value {
        match self {
            TokenValue::String(expr) => expr.get_value(tokens),
            _ => panic!("Can't resolve"),
        }
    }
}
