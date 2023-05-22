use csscolorparser::Color;
use serde::Deserialize;

use crate::expression::Value;

#[derive(Debug, Deserialize)]
pub enum Extensions {
    #[serde(rename = "studio.tokens")]
    StudioTokens(StudioTokensExtension),
}

#[derive(Debug, Deserialize)]
pub enum StudioTokensModify {
    #[serde(rename = "lighten")]
    Lighten,
    #[serde(rename = "darken")]
    Darken,
    #[serde(rename = "alpha")]
    Alpha,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub enum StudioTokensSpace {
    #[serde(rename = "hsl")]
    Hsl,
    #[serde(rename = "lch")]
    Lch,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub enum StudioTokensExtension {
    #[serde(rename = "modify")]
    Modify {
        #[serde(rename = "type")]
        type_: StudioTokensModify,
        value: String,
        space: StudioTokensSpace,
    },
}
impl StudioTokensExtension {
    pub fn to_css(&self, base_value: &Value) -> String {
        match self {
            StudioTokensExtension::Modify {
                type_,
                value,
                space,
            } => {
                let value: f64 = value.parse().unwrap();
                match base_value {
                    Value::Color(color) => match space {
                        StudioTokensSpace::Hsl => {
                            let (h, s, l, a) = color.to_hsla();
                            let l2 = match type_ {
                                StudioTokensModify::Lighten => l + l * value,
                                StudioTokensModify::Darken => l - l * value,
                                _ => panic!("Invalid type: {:?}", type_),
                            };
                            Color::from_hsla(h, s, l2, a).to_hex_string()
                        }
                        StudioTokensSpace::Lch => {
                            let (l, c, h, a) = color.to_lch();
                            let a2 = match type_ {
                                StudioTokensModify::Alpha => a + a * value,
                                _ => panic!("Invalid type: {:?}", type_),
                            };
                            Color::from_lch(l, c, h, a2).to_hex_string()
                        }
                        StudioTokensSpace::Other => todo!(),
                    },
                    _ => panic!("Unexpected base value: {:?}", base_value),
                }
            }
        }
    }
    pub fn to_rust(&self, base_value: &Value) -> Value {
        match self {
            StudioTokensExtension::Modify {
                type_,
                value,
                space,
            } => {
                let value: f64 = value.parse().unwrap();
                match base_value {
                    Value::Color(color) => match space {
                        StudioTokensSpace::Hsl => {
                            let (h, s, l, a) = color.to_hsla();
                            let l2 = match type_ {
                                StudioTokensModify::Lighten => l + l * value,
                                StudioTokensModify::Darken => l - l * value,
                                _ => panic!("Invalid type: {:?}", type_),
                            };
                            Value::Color(Color::from_hsla(h, s, l2, a))
                        }
                        StudioTokensSpace::Lch => {
                            let (l, c, h, a) = color.to_lch();
                            let a2 = match type_ {
                                StudioTokensModify::Alpha => a + a * value,
                                _ => panic!("Invalid type: {:?}", type_),
                            };
                            Value::Color(Color::from_lch(l, c, h, a2))
                        }
                        StudioTokensSpace::Other => todo!(),
                    },
                    _ => panic!("Unexpected base value: {:?}", base_value),
                }
            }
        }
    }
}
