use crate::tools::tool::get_tools;
use anyhow::anyhow;
use async_openai::Client;
use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessageArgs,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
    ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
};
use tracing::{error, info};

pub async fn chat_tool_calculator(
    model: &str,
    system: &str,
    prompt: &str,
) -> anyhow::Result<String> {
    let client = Client::new();
    let mut messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(system)
            .build()?
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(prompt)
            .build()?
            .into(),
    ];
    let (tools, mut tools_map) = get_tools().await?;

    loop {
        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(messages.clone())
            .tools(tools.clone())
            .build()?
            .into();
        let response = client.chat().create(request).await?;

        info!("{:?}", response);

        let message = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no message choices"))?
            .message;

        if let Some(tool_calls) = message.tool_calls {
            messages.push(
                ChatCompletionRequestAssistantMessageArgs::default()
                    .tool_calls(tool_calls.clone())
                    .build()?
                    .into(),
            );
            for tool_call in tool_calls {
                if let ChatCompletionMessageToolCalls::Function(tool_call) = tool_call {
                    let name = tool_call.function.name;
                    let args = tool_call.function.arguments;
                    info!("工具{name}, 参数{args}");
                    let result = match tools_map.get_mut(name.as_str()) {
                        Some(tool) => match tool.execute(args.as_str()).await {
                            Ok(response) => {
                                info!("调用工具{name}成功, 执行结果是{:?}", response);
                                response
                            }
                            Err(err) => {
                                error!("调用工具{name}失败, 执行结果是{:?}", err);
                                format!("执行工具调用失败：{}", err)
                            }
                        },
                        None => {
                            format!("没有找到指定工具{name}")
                        }
                    };
                    // 将结果追加到新的消息中
                    messages.push(
                        ChatCompletionRequestToolMessageArgs::default()
                            .content(result)
                            .tool_call_id(tool_call.id.clone())
                            .build()?
                            .into(),
                    );
                }
            }
        } else {
            let result = message.content.ok_or_else(|| anyhow!("no content"))?;
            info!("最终结果是：{}", result);
            return Ok(result);
        }
    }
}
