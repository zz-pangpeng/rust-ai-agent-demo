mod common;

#[cfg(test)]
mod test {

    use crate::common::client::{ModeChatClientStatus, get_client};
    use crate::common::search_tool::{SearchStatus, get_agent_bind_tool};
    use ai_agent::agent::output::{OUT_MAX_STEPS, TOOL_NOT_FOUND};
    use ai_agent::modals::config::AgentConfig;
    use ai_agent::tools::output::{TOOL_EXECUTE_FAILURE, TOOL_EXECUTE_TIMEOUT};
    #[tokio::test]
    async fn tool_not_found_test() {
        let mut agent = get_client(ModeChatClientStatus::ToolNotFound);
        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().output.contains(TOOL_NOT_FOUND));
    }
    #[tokio::test]
    async fn tool_execute_success_test() {
        let mut agent = get_agent_bind_tool(
            ModeChatClientStatus::ToolExecuteSuccess,
            SearchStatus::Success,
            false,
        );

        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().output, "search content");
    }

    #[tokio::test]
    async fn out_max_steps_test() {
        let mut agent = get_agent_bind_tool(
            ModeChatClientStatus::OutMaxSteps,
            SearchStatus::Success,
            false,
        );

        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().output.contains(OUT_MAX_STEPS));
    }

    #[tokio::test]
    async fn tool_execute_timeout_test() {
        let mut agent = get_agent_bind_tool(
            ModeChatClientStatus::ToolTimeout,
            SearchStatus::Timeout,
            false,
        );

        let mut config = AgentConfig::new();
        config.tool_execute_timeout = 1;

        agent.bind_config(config);

        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().output.contains(TOOL_EXECUTE_TIMEOUT));
    }

    #[tokio::test]
    async fn tool_execute_failure_test() {
        let mut agent = get_agent_bind_tool(
            ModeChatClientStatus::ToolTimeout,
            SearchStatus::Failure,
            false,
        );

        let mut config = AgentConfig::new();
        config.tool_execute_timeout = 1;

        agent.bind_config(config);

        let result = agent.run("".to_string()).await;

        assert!(result.is_ok());
        assert!(result.unwrap().output.contains(TOOL_EXECUTE_FAILURE));
    }
}
