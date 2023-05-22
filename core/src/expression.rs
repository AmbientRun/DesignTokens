use std::fmt;

use csscolorparser::Color;
use itertools::Itertools;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};

use crate::{slugify_css, DesignTokens};

#[derive(Debug, Clone, PartialEq)]
pub enum NumberType {
    None,
    Pixels,
    Percentage,
}
impl NumberType {
    fn to_css(&self, value: f32) -> String {
        match self {
            NumberType::None => format!("{}", value),
            NumberType::Pixels => format!("{}px", value),
            NumberType::Percentage => format!("{}%", value),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Color(Color),
    Number(f32, NumberType),
    Any(String),
}
impl Value {
    pub fn to_css(&self) -> String {
        match self {
            Value::Color(val) => val.to_hex_string(),
            Value::Number(val, typ) => typ.to_css(*val),
            Value::Any(val) => val.to_string(),
        }
    }
    pub fn to_rust(&self) -> String {
        match self {
            Value::Color(val) => val.to_hex_string(),
            Value::Number(val, typ) => typ.to_css(*val),
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
                format!("var(--{})", path.iter().map(|x| slugify_css(x)).join("--"))
            }
            Expression::Mul(a, b) => format!("calc({} * {})", a.to_css(tokens), b.to_css(tokens)),
            Expression::Div(a, b) => format!("calc({} / {})", a.to_css(tokens), b.to_css(tokens)),
            Expression::Value(val) => val.to_css(),
        }
    }
    pub fn to_rust(&self, tokens: &DesignTokens) -> String {
        match self {
            Expression::Ref(path) => tokens.get_value(path).get_value(tokens).to_rust(),
            Expression::Mul(a, b) => format!("{} * {}", a.to_rust(tokens), b.to_rust(tokens)),
            Expression::Div(a, b) => format!("{} / {}", a.to_rust(tokens), b.to_rust(tokens)),
            Expression::Value(val) => val.to_rust(),
        }
    }
    pub fn get_value(&self, tokens: &DesignTokens) -> Value {
        match self {
            Expression::Ref(path) => tokens.get_value(path).get_value(tokens),
            Expression::Mul(a, b) => match (a.get_value(tokens), b.get_value(tokens)) {
                (Value::Color(a), Value::Color(b)) => Value::Color(Color {
                    r: a.r * b.r,
                    g: a.g * b.g,
                    b: a.b * b.b,
                    a: a.a * b.a,
                }),
                (Value::Number(a, typ), Value::Number(b, _)) => Value::Number(a * b, typ),
                (a, b) => todo!("Not handled: {:?} {:?}", a, b),
            },
            Expression::Div(a, b) => match (a.get_value(tokens), b.get_value(tokens)) {
                (Value::Color(a), Value::Color(b)) => Value::Color(Color {
                    r: a.r / b.r,
                    g: a.g / b.g,
                    b: a.b / b.b,
                    a: a.a / b.a,
                }),
                (Value::Number(a, typ), Value::Number(b, _)) => Value::Number(a / b, typ),
                _ => todo!(),
            },
            Expression::Value(value) => value.clone(),
        }
    }
}

peg::parser! {
  grammar expr_parser() for str {
    rule _ = quiet!{[' ' | '\n' | '\t']*}

    rule number() -> f32
        = n:$("-"? ['0'..='9']+ "."? ['0'..='9']*) {? n.parse().or(Err("f32")) }

    pub(crate) rule expr() -> Expression = precedence!{
        x:(@) _ "*" _ y:@ { Expression::Mul(Box::new(x), Box::new(y)) }
        x:(@) _ "/" _ y:@ { Expression::Div(Box::new(x), Box::new(y)) }
        --
        "{" v:($((!"}" !"." [_])*) ** ".") "}" { Expression::Ref(v.iter().map(|x| x.to_string()).collect()) }
        "#" v:$(['a'..='z' | 'A'..='Z' | '0'..='9']*) { Expression::Value(Value::Color(csscolorparser::parse(v).unwrap())) }
        v:number() "%" { Expression::Value(Value::Number(v, NumberType::Percentage)) }
        v:number() "px" { Expression::Value(Value::Number(v, NumberType::Pixels)) }
        v:number() { Expression::Value(Value::Number(v, NumberType::None)) }
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
        Expression::Value(Value::Number(90., NumberType::Percentage))
    );
    assert_eq!(
        expr_parser::expr("-90%").unwrap(),
        Expression::Value(Value::Number(-90., NumberType::Percentage))
    );
    assert_eq!(
        expr_parser::expr("ABC Diatype Variable").unwrap(),
        Expression::Value(Value::Any("ABC Diatype Variable".to_string()))
    );
    assert_eq!(
        expr_parser::expr("232.8300018310547").unwrap(),
        Expression::Value(Value::Number(232.8300018310547, NumberType::None))
    );
    assert_eq!(
        expr_parser::expr("2px").unwrap(),
        Expression::Value(Value::Number(2., NumberType::Pixels))
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
            Box::new(Expression::Value(Value::Number(5., NumberType::None))),
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
