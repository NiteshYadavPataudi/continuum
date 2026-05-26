use continuum_core::model::CostEstimate;
use continuum_models_registry::MODELS;

pub fn estimate_cost(
    provider: &str,
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
) -> CostEstimate {
    let full_key = format!("{provider}/{model}");
    let (input_price, output_price) = MODELS
        .get(full_key.as_str())
        .map(|m| (m.input_per_mtok, m.output_per_mtok))
        .unwrap_or((3.0, 15.0));

    let usd = (input_tokens as f64 * input_price / 1_000_000.0)
        + (output_tokens as f64 * output_price / 1_000_000.0);

    CostEstimate::new(input_tokens, output_tokens, usd)
}
