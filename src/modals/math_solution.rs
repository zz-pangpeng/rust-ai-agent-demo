use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct MathSolution {
    pub final_answer: String,
    pub steps: Vec<Step>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Step {
    pub output: String,
    pub explanation: String,
}
