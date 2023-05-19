use std::fmt;

use csscolorparser::Color;
use itertools::Itertools;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};
use slug::slugify;

use crate::DesignTokens;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Color(Color),
    Any(String),
}
impl Value {
    pub fn to_css(&self) -> String {
        match self {
            Value::Color(val) => val.to_hex_string(),
            Value::Any(val) => val.to_string(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Expression {
    Ref(Vec<String>),
    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),
    Value(Value),
}
impl Expression {
    pub fn to_css(&self, tokens: &DesignTokens) -> String {
        match self {
            Expression::Ref(path) => {
                format!("var(--{})", path.iter().map(|x| slugify(x)).join("--"))
            }
            Expression::Mul(a, b) => format!("calc({} * {})", a.to_css(tokens), b.to_css(tokens)),
            Expression::Div(a, b) => format!("calc({} / {})", a.to_css(tokens), b.to_css(tokens)),
            Expression::Value(val) => val.to_css(),
        }
    }
    pub fn get_value(&self, tokens: &DesignTokens) -> Value {
        match self {
            Expression::Ref(path) => tokens.get_value(path).get_value(tokens),
            Expression::Mul(_, _) => todo!(),
            Expression::Div(_, _) => todo!(),
            Expression::Value(value) => value.clone(),
        }
    }
}

peg::parser! {
  grammar expr_parser() for str {
    rule _ = quiet!{[' ' | '\n' | '\t']*}


    pub(crate) rule expr() -> Expression = precedence!{
        x:(@) _ "*" _ y:@ { Expression::Mul(Box::new(x), Box::new(y)) }
        x:(@) _ "/" _ y:@ { Expression::Div(Box::new(x), Box::new(y)) }
        --
        "{" v:($((!"}" !"." [_])*) ** ".") "}" { Expression::Ref(v.iter().map(|x| x.to_string()).collect()) }
        "#" v:$(['a'..='z' | 'A'..='Z' | '0'..='9']*) { Expression::Value(Value::Color(csscolorparser::parse(v).unwrap())) }
        v:$(['a'..='z' | 'A'..='Z' | '0'..='9' | '#' | '%' | '-' | '.' | ' ']*) { Expression::Value(Value::Any(v.to_string())) }
    }
  }
}

#[test]
fn test() {
    assert_eq!(
        expr_parser::expr("{hello.world}").unwrap(),
        Expression::Ref(vec!["hello".to_string(), "world".to_string()])
    );
    assert_eq!(
        expr_parser::expr("#ff00ff").unwrap(),
        Expression::Value(Value::Color(csscolorparser::parse("#ff00ff").unwrap()))
    );
    assert_eq!(
        expr_parser::expr("90%").unwrap(),
        Expression::Value(Value::Any("90%".to_string()))
    );
    assert_eq!(
        expr_parser::expr("-90%").unwrap(),
        Expression::Value(Value::Any("-90%".to_string()))
    );
    assert_eq!(
        expr_parser::expr("ABC Diatype Variable").unwrap(),
        Expression::Value(Value::Any("ABC Diatype Variable".to_string()))
    );
    assert_eq!(
        expr_parser::expr("232.8300018310547").unwrap(),
        Expression::Value(Value::Any("232.8300018310547".to_string()))
    );

    assert_eq!(
        expr_parser::expr("{x} * {y}").unwrap(),
        Expression::Mul(
            Box::new(Expression::Ref(vec!["x".to_string()])),
            Box::new(Expression::Ref(vec!["y".to_string()])),
        )
    );
    assert_eq!(
        expr_parser::expr("{x}/5").unwrap(),
        Expression::Div(
            Box::new(Expression::Ref(vec!["x".to_string()])),
            Box::new(Expression::Value(Value::Any("5".to_string()))),
        )
    );
}

struct ExpressionVisitor;

impl<'de> Visitor<'de> for ExpressionVisitor {
    type Value = Expression;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match expr_parser::expr(value) {
            Ok(expr) => Ok(expr),
            Err(err) => Err(E::custom(format!("Invalid expression: {}", err))),
        }
    }
}

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Expression, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(ExpressionVisitor)
    }
}
