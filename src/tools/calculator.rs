use crate::modals::calculator::CalculatorArg;
use crate::tools::tool::Tool;
use anyhow::anyhow;
use schemars::schema_for;

pub struct CalculatorTool;
#[async_trait::async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &'static str {
        "calculator"
    }

    fn description(&self) -> &'static str {
        "两数之间的加减乘除计算, 支持的operator有: '+', '-', '*', '/'"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::to_value(schema_for!(CalculatorArg)).expect("Can't serialize CalculatorArg")
    }

    async fn execute(&mut self, args: &str) -> anyhow::Result<String> {
        let arg: CalculatorArg = serde_json::from_str(args)?;
        match arg.operator.to_lowercase().as_str() {
            "+" => Ok(format!(
                "{} + {} = {}",
                arg.first_number,
                arg.second_number,
                arg.first_number + arg.second_number
            )),
            "-" => Ok(format!(
                "{} - {} = {}",
                arg.first_number,
                arg.second_number,
                arg.first_number - arg.second_number
            )),
            "*" => Ok(format!(
                "{} * {} = {}",
                arg.first_number,
                arg.second_number,
                arg.first_number * arg.second_number
            )),
            "/" => {
                if arg.second_number == 0 {
                    return Err(anyhow!("second number cannot be 0".to_string()));
                }
                Ok(format!(
                    "{} / {} = {}",
                    arg.first_number,
                    arg.second_number,
                    arg.first_number / arg.second_number
                ))
            }
            other => Err(anyhow!("operator {} is not supported", other)),
        }
    }
}
