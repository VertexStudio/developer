use rmcp::{Error as McpError, model::CallToolResult, model::Content};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_description: String,
    pub step_number: i32,
    pub total_steps: i32,
    pub next_step_needed: bool,
    pub is_step_revision: Option<bool>,
    pub revises_step: Option<i32>,
    pub branch_from_step: Option<i32>,
    pub branch_id: Option<String>,
    pub needs_more_steps: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkflowStatus {
    step_number: i32,
    total_steps: i32,
    next_step_needed: bool,
    last_step_description: String,
    current_branch: Option<String>,
    branches: Vec<String>,
    step_history_length: usize,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct WorkflowState {
    step_history: Vec<WorkflowStep>,
    branches: HashMap<String, Vec<WorkflowStep>>,
    current_branch: Option<String>,
}

#[derive(Clone)]
pub struct Workflow {
    state: Arc<Mutex<WorkflowState>>,
    allow_branches: bool,
    max_steps: Option<i32>,
}

impl Default for Workflow {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(WorkflowState::default())),
            allow_branches: true,
            max_steps: None,
        }
    }
}

impl Workflow {
    pub fn new(allow_branches: bool, max_steps: Option<i32>) -> Self {
        Self {
            state: Arc::new(Mutex::new(WorkflowState::default())),
            allow_branches,
            max_steps,
        }
    }

    pub async fn execute_step(&self, args: WorkflowStep) -> Result<CallToolResult, McpError> {
        if let Some(max) = self.max_steps {
            if args.step_number > max {
                return Ok(Self::error(format!(
                    "Step number {} exceeds configured maximum of {}",
                    args.step_number, max
                )));
            }
        }

        let mut state = self.state.lock().await;

        let mut step_data = args.clone();
        if step_data.step_number > step_data.total_steps {
            step_data.total_steps = step_data.step_number;
        }

        if step_data.revises_step.is_some() && step_data.is_step_revision.is_none() {
            return Ok(Self::error(
                "When specifying revises_step, is_step_revision must be set to true",
            ));
        }

        if step_data.branch_id.is_some() && step_data.branch_from_step.is_none() {
            return Ok(Self::error(
                "When creating a branch (branch_id), you must specify branch_from_step",
            ));
        }

        if let (Some(branch_id), Some(branch_from_step)) =
            (&step_data.branch_id, &step_data.branch_from_step)
        {
            if !self.allow_branches {
                return Ok(Self::error(
                    "Branching is disabled in current configuration",
                ));
            }

            if *branch_from_step <= 0 || *branch_from_step > state.step_history.len() as i32 {
                return Ok(Self::error(format!(
                    "branch_from_step {} does not exist in step history",
                    branch_from_step
                )));
            }

            state.current_branch = Some(branch_id.clone());
            state
                .branches
                .entry(branch_id.clone())
                .or_default()
                .push(step_data.clone());
        } else if state.current_branch.is_some() && step_data.branch_id.is_none() {
            state.current_branch = None;
        }

        state.step_history.push(step_data.clone());

        let response = self.build_workflow_status(&state, &step_data).await;

        match serde_json::to_string_pretty(&response) {
            Ok(json_response) => Ok(Self::success(json_response)),
            Err(e) => Ok(Self::error(format!("Failed to serialize response: {}", e))),
        }
    }

    fn error(error_message: impl Into<String>) -> CallToolResult {
        CallToolResult::error(vec![Content::text(error_message.into())])
    }

    fn success(message: impl Into<String>) -> CallToolResult {
        CallToolResult::success(vec![Content::text(message.into())])
    }

    async fn build_workflow_status(
        &self,
        state: &WorkflowState,
        step_data: &WorkflowStep,
    ) -> WorkflowStatus {
        WorkflowStatus {
            step_number: step_data.step_number,
            total_steps: step_data.total_steps,
            next_step_needed: step_data.next_step_needed,
            last_step_description: step_data.step_description.clone(),
            current_branch: state.current_branch.clone(),
            branches: state.branches.keys().cloned().collect(),
            step_history_length: state.step_history.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_tool() {
        let tool = Workflow::default();
        let step = WorkflowStep {
            step_description: "Initial step".to_string(),
            step_number: 1,
            total_steps: 3,
            next_step_needed: true,
            is_step_revision: None,
            revises_step: None,
            branch_from_step: None,
            branch_id: None,
            needs_more_steps: None,
        };

        let result = tool.execute_step(step).await.unwrap();
        // Check that we got a successful result
        assert!(result.is_error.is_none() || result.is_error == Some(false));

        // Parse the response to verify structure
        if let Some(content) = result.content.first() {
            if let Some(text_content) = content.as_text() {
                let response: Result<WorkflowStatus, _> = serde_json::from_str(&text_content.text);
                assert!(response.is_ok());
                let status = response.unwrap();
                assert_eq!(status.step_number, 1);
                assert_eq!(status.total_steps, 3);
                assert_eq!(status.next_step_needed, true);
                assert_eq!(status.step_history_length, 1);
                assert!(status.branches.is_empty());
            }
        }
    }

    #[tokio::test]
    async fn test_workflow_branching() {
        let tool = Workflow::default();

        let step1 = WorkflowStep {
            step_description: "Initial step".to_string(),
            step_number: 1,
            total_steps: 3,
            next_step_needed: true,
            is_step_revision: None,
            revises_step: None,
            branch_from_step: None,
            branch_id: None,
            needs_more_steps: None,
        };
        let _ = tool.execute_step(step1).await.unwrap();

        let branch_step = WorkflowStep {
            step_description: "Branch step".to_string(),
            step_number: 2,
            total_steps: 3,
            next_step_needed: true,
            is_step_revision: None,
            revises_step: None,
            branch_from_step: Some(1),
            branch_id: Some("test_branch".to_string()),
            needs_more_steps: None,
        };

        let result = tool.execute_step(branch_step).await.unwrap();

        // Parse and verify the branching response
        if let Some(content) = result.content.first() {
            if let Some(text_content) = content.as_text() {
                let response: Result<WorkflowStatus, _> = serde_json::from_str(&text_content.text);
                assert!(response.is_ok());
                let status = response.unwrap();
                assert_eq!(status.current_branch, Some("test_branch".to_string()));
                assert_eq!(status.branches.len(), 1);
                assert!(status.branches.contains(&"test_branch".to_string()));
            }
        }
    }

    #[test]
    fn test_workflow_creation() {
        let tool = Workflow::new(true, Some(10));
        assert_eq!(tool.allow_branches, true);
        assert_eq!(tool.max_steps, Some(10));

        let default_tool = Workflow::default();
        assert_eq!(default_tool.allow_branches, true);
        assert_eq!(default_tool.max_steps, None);
    }

    #[tokio::test]
    async fn test_error_conditions() {
        let tool = Workflow::new(false, Some(2)); // No branching, max 2 steps

        // Test max steps exceeded
        let step = WorkflowStep {
            step_description: "Too many steps".to_string(),
            step_number: 3,
            total_steps: 5,
            next_step_needed: true,
            is_step_revision: None,
            revises_step: None,
            branch_from_step: None,
            branch_id: None,
            needs_more_steps: None,
        };

        let result = tool.execute_step(step).await.unwrap();
        assert!(result.is_error == Some(true));

        // Test branching disabled
        let step1 = WorkflowStep {
            step_description: "Initial step".to_string(),
            step_number: 1,
            total_steps: 2,
            next_step_needed: true,
            is_step_revision: None,
            revises_step: None,
            branch_from_step: None,
            branch_id: None,
            needs_more_steps: None,
        };
        let _ = tool.execute_step(step1).await.unwrap();

        let branch_step = WorkflowStep {
            step_description: "Branch step".to_string(),
            step_number: 2,
            total_steps: 2,
            next_step_needed: false,
            is_step_revision: None,
            revises_step: None,
            branch_from_step: Some(1),
            branch_id: Some("test_branch".to_string()),
            needs_more_steps: None,
        };

        let result = tool.execute_step(branch_step).await.unwrap();
        assert!(result.is_error == Some(true));
    }
}
