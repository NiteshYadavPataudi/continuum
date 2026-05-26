use continuum_core::agent::AgentKind;
use continuum_core::planner::{ExecutionPlan, PlanError, PlanEstimate};

struct NodeCost {
    input_tokens: u64,
    output_tokens: u64,
    runtime_secs: u64,
    risk_delta: f32,
}

fn cost_for_kind(kind: AgentKind) -> NodeCost {
    match kind {
        AgentKind::Architecture => NodeCost {
            input_tokens: 1200,
            output_tokens: 500,
            runtime_secs: 25,
            risk_delta: 0.05,
        },
        AgentKind::Coding => NodeCost {
            input_tokens: 2000,
            output_tokens: 800,
            runtime_secs: 60,
            risk_delta: 0.10,
        },
        AgentKind::Testing => NodeCost {
            input_tokens: 1500,
            output_tokens: 600,
            runtime_secs: 45,
            risk_delta: 0.05,
        },
        AgentKind::Security => NodeCost {
            input_tokens: 1000,
            output_tokens: 400,
            runtime_secs: 30,
            risk_delta: 0.02,
        },
        AgentKind::Review => NodeCost {
            input_tokens: 800,
            output_tokens: 350,
            runtime_secs: 20,
            risk_delta: 0.02,
        },
        AgentKind::Planner => NodeCost {
            input_tokens: 600,
            output_tokens: 250,
            runtime_secs: 15,
            risk_delta: 0.08,
        },
        AgentKind::Recovery => NodeCost {
            input_tokens: 500,
            output_tokens: 200,
            runtime_secs: 10,
            risk_delta: 0.12,
        },
        AgentKind::Memory => NodeCost {
            input_tokens: 300,
            output_tokens: 100,
            runtime_secs: 5,
            risk_delta: 0.00,
        },
    }
}

/// Compute a [`PlanEstimate`] using per-agent-kind token/time/risk tables.
pub async fn estimate(plan: &ExecutionPlan) -> Result<PlanEstimate, PlanError> {
    let mut input_tokens: u64 = 0;
    let mut output_tokens: u64 = 0;
    let mut runtime_secs: u64 = 0;
    let mut risk: f32 = 0.05; // base risk

    for node in &plan.nodes {
        let c = cost_for_kind(node.agent_kind);
        input_tokens += c.input_tokens;
        output_tokens += c.output_tokens;
        runtime_secs += c.runtime_secs;
        risk += c.risk_delta;
    }

    // USD: rough model pricing at ~$3/MTok input, ~$15/MTok output (Sonnet-class).
    let usd = (input_tokens as f64 / 1_000_000.0) * 3.0
        + (output_tokens as f64 / 1_000_000.0) * 15.0;

    let mut est = PlanEstimate::default();
    est.input_tokens = input_tokens;
    est.output_tokens = output_tokens;
    est.runtime_secs = runtime_secs;
    est.usd = usd;
    est.risk = risk.min(1.0);
    Ok(est)
}
