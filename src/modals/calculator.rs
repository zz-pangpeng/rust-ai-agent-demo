use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
pub struct CalculatorArg {
    pub first_number: i32,
    pub second_number: i32,
    pub operator: String,
}