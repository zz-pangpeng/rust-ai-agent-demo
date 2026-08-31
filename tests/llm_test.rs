mod common;
use common::*;

#[cfg(test)]
mod tests {
    use ai_agent::agent::output::{NOT_OUTPUT, TIMEOUT};
    use ai_agent::modals::config::AgentConfig;
    use super::*;

    #[tokio::test]
    async fn llm_output_content_test() {
        let mut agent =get_client(ModeChatClientStatus::Normal);
        
        let result = agent.run("hello".to_string()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().output, "hello rust agent");
    }

    #[tokio::test]
    async fn llm_no_output_test() {
        let mut agent =get_client(ModeChatClientStatus::NotOutput);
        
        let result = agent.run("hello".to_string()).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains(NOT_OUTPUT));
        }
    }

    #[tokio::test]
    async fn llm_choice_empty_test() {
        let mut agent =get_client(ModeChatClientStatus::ChoiceEmpty);
        
        let result = agent.run("hello".to_string()).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains(NOT_OUTPUT));
        }
    }

    #[tokio::test]
    async fn llm_timeout_test() {
        let mut agent =get_client(ModeChatClientStatus::Timeout);

        let mut config = AgentConfig::new();
        config.agent_execute_timeout = 1;
        config.agent_execute_retry_count = 1;
        agent.bind_config(config);


        let result = agent.run("hello".to_string()).await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains(TIMEOUT));
        }
    }
}