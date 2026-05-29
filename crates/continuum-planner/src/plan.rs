use std::sync::Arc;

use continuum_core::agent::AgentKind;
use continuum_core::caps::Cap;
use continuum_core::ids::{ModelId, TaskId};
use continuum_core::model::{CompletionRequest, Message, ModelProvider};
use continuum_core::planner::{Dependency, ExecutionPlan, Goal, PlanError, RepoAnalysis, TaskNode};
use futures::StreamExt;

/// Decompose a [`Goal`] into an [`ExecutionPlan`] with a real DAG.
///
/// When a model provider is given, uses a two-step LLM decomposition.
/// When no provider is available, falls back to keyword-driven heuristics.
pub async fn plan(
    goal: Goal,
    analysis: &RepoAnalysis,
    model: Option<Arc<dyn ModelProvider>>,
    model_id: ModelId,
) -> Result<ExecutionPlan, PlanError> {
    if let Some(provider) = model {
        plan_with_model(goal, analysis, provider, model_id).await
    } else {
        plan_heuristic(goal)
    }
}

async fn plan_with_model(
    goal: Goal,
    analysis: &RepoAnalysis,
    provider: Arc<dyn ModelProvider>,
    model_id: ModelId,
) -> Result<ExecutionPlan, PlanError> {
    let constraints_str = if goal.constraints.is_null() {
        "none".to_string()
    } else {
        goal.constraints.to_string()
    };

    let prompt = format!(
        r#"You are a software-project planner. Decompose the following goal into an ordered list of tasks.

Each task must specify which agent should handle it and a short label.
Available agent kinds: Coding, Testing, Architecture, Security, Review, Recovery, Planner

Return ONLY valid JSON with no markdown fences, matching this schema exactly:
{{
  "tasks": [
    {{
      "agent": "<one of the agent kinds above>",
      "label": "<one concise sentence describing what this task accomplishes>",
      "depends_on": [<list of 0-based indices of tasks this must wait for, or empty array>]
    }}
  ]
}}

Repository context: {summary}
Goal: {goal}
Constraints: {constraints}

Rules:
- Always include at least one Coding task.
- Place Architecture tasks before Coding tasks that implement the design.
- Place Testing tasks after the Coding tasks they cover.
- Place Review tasks last, after Coding and Testing.
- Security tasks can run in parallel with Review.
- Keep the plan concise: 2–6 tasks for simple goals, up to 10 for complex ones.
- depends_on must only reference earlier indices (no cycles)."#,
        summary = analysis.summary,
        goal = goal.prompt,
        constraints = constraints_str,
    );

    let req = CompletionRequest::new(model_id, vec![Message::new("user", &prompt)]);

    let cancel = continuum_core::CancellationToken::new();
    let mut stream = provider
        .complete(&Cap::grant(), req, cancel.child_token())
        .await
        .map_err(|e| PlanError::Model(e.to_string()))?;

    let mut raw = String::new();
    while let Some(delta) = stream.next().await {
        match delta {
            Ok(d) => raw.push_str(&d.text),
            Err(e) => return Err(PlanError::Model(format!("stream error: {e}"))),
        }
    }

    parse_llm_plan(&raw, &goal.prompt)
}

/// Parse the JSON response from the LLM into an `ExecutionPlan`.
fn parse_llm_plan(raw: &str, goal_prompt: &str) -> Result<ExecutionPlan, PlanError> {
    // Strip accidental markdown fences the model might emit.
    let json_str = strip_fences(raw);

    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| PlanError::Other(format!("model returned invalid JSON: {e}\nraw: {raw}")))?;

    let tasks = value
        .get("tasks")
        .and_then(|v| v.as_array())
        .ok_or_else(|| PlanError::Other("model JSON missing 'tasks' array".into()))?;

    if tasks.is_empty() {
        return Err(PlanError::InvalidGoal(
            "model returned zero tasks for goal".into(),
        ));
    }

    let mut nodes: Vec<TaskNode> = Vec::with_capacity(tasks.len());
    let mut raw_deps: Vec<Vec<usize>> = Vec::with_capacity(tasks.len());

    for (i, t) in tasks.iter().enumerate() {
        let agent_str = t
            .get("agent")
            .and_then(|v| v.as_str())
            .ok_or_else(|| PlanError::Other(format!("task {i} missing 'agent'")))?;

        let agent_kind = parse_agent_kind(agent_str).ok_or_else(|| {
            PlanError::Other(format!("unknown agent kind '{agent_str}' in task {i}"))
        })?;

        let label = t
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or(goal_prompt)
            .to_string();

        let deps: Vec<usize> = t
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_u64().map(|n| n as usize))
                    .collect()
            })
            .unwrap_or_default();

        let id = TaskId::new();
        let node: TaskNode = serde_json::from_value(serde_json::json!({
            "id": id,
            "agent_kind": agent_kind,
            "payload": { "goal": goal_prompt, "label": &label },
            "label": &label,
        }))
        .map_err(|e| PlanError::Other(format!("failed to build task node: {e}")))?;
        nodes.push(node);
        raw_deps.push(deps);
    }

    // Build edges from deps, validating indices.
    let mut edges: Vec<(TaskId, TaskId, Dependency)> = Vec::new();
    for (i, deps) in raw_deps.iter().enumerate() {
        for &dep_idx in deps {
            if dep_idx >= nodes.len() || dep_idx >= i {
                // Skip invalid or forward references (prevents cycles).
                continue;
            }
            edges.push((nodes[dep_idx].id, nodes[i].id, Dependency::HappensBefore));
        }
    }

    // If no edges were produced (model gave no depends_on), chain nodes sequentially.
    if edges.is_empty() && nodes.len() > 1 {
        for i in 1..nodes.len() {
            edges.push((nodes[i - 1].id, nodes[i].id, Dependency::HappensBefore));
        }
    }

    let mut plan = ExecutionPlan::default();
    plan.nodes = nodes;
    plan.edges = edges;
    Ok(plan)
}

fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    // Strip ```json ... ``` or ``` ... ```
    if let Some(inner) = s.strip_prefix("```json").or_else(|| s.strip_prefix("```")) {
        if let Some(end) = inner.rfind("```") {
            return inner[..end].trim();
        }
    }
    s
}

fn parse_agent_kind(s: &str) -> Option<AgentKind> {
    match s.to_lowercase().as_str() {
        "coding" | "code" => Some(AgentKind::Coding),
        "testing" | "test" => Some(AgentKind::Testing),
        "architecture" | "arch" => Some(AgentKind::Architecture),
        "security" | "sec" => Some(AgentKind::Security),
        "review" => Some(AgentKind::Review),
        "recovery" => Some(AgentKind::Recovery),
        "planner" | "planning" => Some(AgentKind::Planner),
        "memory" => Some(AgentKind::Memory),
        _ => None,
    }
}

/// Keyword-heuristic fallback used when no model is configured.
fn plan_heuristic(goal: Goal) -> Result<ExecutionPlan, PlanError> {
    let text = goal.prompt.trim().to_lowercase();
    let mut nodes: Vec<TaskNode> = Vec::new();

    // Architecture first when goal is design-oriented.
    if text.contains("architect")
        || text.contains("design")
        || text.contains("refactor")
        || text.contains("restructure")
    {
        nodes.push(make_node(
            AgentKind::Architecture,
            "Review and define architectural boundaries",
            &goal.prompt,
        ));
    }

    // Security audit before coding when explicitly requested.
    if text.contains("security") || text.contains("audit") || text.contains("harden") {
        nodes.push(make_node(
            AgentKind::Security,
            "Run security audit and identify vulnerabilities",
            &goal.prompt,
        ));
    }

    // Core coding task — always present.
    nodes.push(make_node(
        AgentKind::Coding,
        &format!("Implement: {}", goal.prompt),
        &goal.prompt,
    ));

    // Tests after coding when goal mentions testing.
    if text.contains("test")
        || text.contains("spec")
        || text.contains("coverage")
        || text.contains("tdd")
    {
        nodes.push(make_node(
            AgentKind::Testing,
            "Write tests for the implemented functionality",
            &goal.prompt,
        ));
    }

    // Review at the end.
    if text.contains("review") || text.contains("pr") || text.contains("refactor") {
        nodes.push(make_node(
            AgentKind::Review,
            "Code review and quality pass",
            &goal.prompt,
        ));
    }

    // Chain all nodes sequentially (simple linear plan).
    let mut edges: Vec<(TaskId, TaskId, Dependency)> = Vec::new();
    for i in 1..nodes.len() {
        edges.push((nodes[i - 1].id, nodes[i].id, Dependency::HappensBefore));
    }

    let mut plan = ExecutionPlan::default();
    plan.nodes = nodes;
    plan.edges = edges;
    Ok(plan)
}

fn make_node(kind: AgentKind, label: &str, goal: &str) -> TaskNode {
    let id = TaskId::new();
    serde_json::from_value(serde_json::json!({
        "id": id,
        "agent_kind": kind,
        "payload": { "goal": goal, "label": label },
        "label": label,
    }))
    .expect("TaskNode schema is always valid here")
}
