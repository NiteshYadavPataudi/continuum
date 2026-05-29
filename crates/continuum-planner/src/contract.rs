use continuum_core::agent::AgentKind;
use continuum_core::planner::{ExecutionContract, ExecutionPlan, PlanError, PlanEstimate};

/// Generate a human-approvable [`ExecutionContract`] from a plan and its estimate.
pub async fn contract(
    plan: &ExecutionPlan,
    estimate: &PlanEstimate,
) -> Result<ExecutionContract, PlanError> {
    let mut summary: Vec<String> = Vec::new();

    // Budget warnings.
    if estimate.usd > 0.50 {
        summary.push(format!(
            "⚠ Estimated cost ${:.2} exceeds $0.50 budget threshold.",
            estimate.usd
        ));
    }
    if estimate.risk > 0.5 {
        summary.push(format!(
            "⚠ Risk score {:.2} is above 0.5 — review plan carefully.",
            estimate.risk
        ));
    }

    // Group nodes by phase for readability.
    let planning_nodes: Vec<_> = plan
        .nodes
        .iter()
        .filter(|n| matches!(n.agent_kind, AgentKind::Architecture | AgentKind::Planner))
        .collect();
    let impl_nodes: Vec<_> = plan
        .nodes
        .iter()
        .filter(|n| matches!(n.agent_kind, AgentKind::Coding | AgentKind::Recovery))
        .collect();
    let validation_nodes: Vec<_> = plan
        .nodes
        .iter()
        .filter(|n| matches!(n.agent_kind, AgentKind::Testing | AgentKind::Security))
        .collect();
    let review_nodes: Vec<_> = plan
        .nodes
        .iter()
        .filter(|n| matches!(n.agent_kind, AgentKind::Review | AgentKind::Memory))
        .collect();

    let mut step = 1usize;

    if !planning_nodes.is_empty() {
        summary.push("── Planning ──".into());
        for n in planning_nodes {
            summary.push(format!(
                "  Step {} — [{:?}] {}",
                step, n.agent_kind, n.label
            ));
            step += 1;
        }
    }

    if !impl_nodes.is_empty() {
        summary.push("── Implementation ──".into());
        for n in impl_nodes {
            summary.push(format!(
                "  Step {} — [{:?}] {}",
                step, n.agent_kind, n.label
            ));
            step += 1;
        }
    }

    if !validation_nodes.is_empty() {
        summary.push("── Validation ──".into());
        for n in validation_nodes {
            summary.push(format!(
                "  Step {} — [{:?}] {}",
                step, n.agent_kind, n.label
            ));
            step += 1;
        }
    }

    if !review_nodes.is_empty() {
        summary.push("── Review ──".into());
        for n in review_nodes {
            summary.push(format!(
                "  Step {} — [{:?}] {}",
                step, n.agent_kind, n.label
            ));
            step += 1;
        }
    }

    // Footer.
    summary.push(format!(
        "── Totals: {} tokens in / {} tokens out / {}s / ${:.4} ──",
        estimate.input_tokens, estimate.output_tokens, estimate.runtime_secs, estimate.usd,
    ));

    let mut contract = ExecutionContract::default();
    contract.plan = plan.clone();
    contract.estimate = estimate.clone();
    contract.summary = summary;
    Ok(contract)
}
