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
    fn to_rust(&self, value: f32) -> String {
        let value = match self {
            NumberType::Percentage => value * 0.01,
            _ => value,
        };
        let x = format!("{}", value);
        if x.contains(".") {
            x
        } else {
            format!("{}.", x)
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
            Value::Color(val) => format!("\"{}\"", val.to_hex_string()),
            Value::Number(val, typ) => typ.to_rust(*val),
            Value::Any(val) => format!("\"{}\"", val.to_string()),
        }
    }
    pub fn to_rust_type(&self) -> &'static str {
        match self {
            Value::Number(_, _) => "f32",
            _ => "&'static str",
        }
    }
    pub fn to_rust_string(&self) -> String {
        match self {
            Value::Number(_, _) => format!("\"{}\"", self.to_rust()),
            _ => self.to_rust(),
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
                format!("var(--{})", path.iter().map(|x| slugify_css(x)).join("-"))
            }
            Expression::Mul(a, b) => format!("calc({} * {})", a.to_css(tokens), b.to_css(tokens)),
            Expression::Div(a, b) => format!("calc({} / {})", a.to_css(tokens), b.to_css(tokens)),
            Expression::Value(val) => val.to_css(),
        }
    }
    pub fn get_value(&self, tokens: &DesignTokens) -> Value {
        match self {
            Expression::Ref(path) => {
                // let path = path.iter().map(|s| slugify_css(s)).collect_vec();
                tokens
                    .get_value(&path)
                    .expect(&format!("No such path: {:?}", path))
                    .get_value(tokens)
            }
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
        "{" v:($((!"}" !"." [_])*) ** ".") "}" { Expression::Ref(v.iter().flat_map(|x| x.to_string().split("/").map(|x| x.to_string()).collect_vec()).collect()) }
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

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Expression::Value(Value::Number(v as f32, NumberType::None)))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Expression::Value(Value::Number(v as f32, NumberType::None)))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Expression::Value(Value::Number(v as f32, NumberType::None)))
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Expression::Value(Value::Number(v as f32, NumberType::None)))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Expression::Value(Value::Number(v as f32, NumberType::None)))
    }
}

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Expression, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ExpressionVisitor)
    }
}

#[test]
fn test_expr() {
    let _expr: Expression = serde_json::from_str("5.5").unwrap();
    let _expr: Expression = serde_json::from_str("55").unwrap();
    let _expr: Expression = serde_json::from_str("\"55\"").unwrap();
}
