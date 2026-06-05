use tokio::sync::broadcast;

use continuum_core::{
    caps::Cap,
    model::{CompletionRequest, Message},
    CancellationToken,
};
use continuum_telemetry::DashboardEvent;

use crate::{
    repl::ReplSession,
    tui::app::{ChatMessage, MessageRole},
};

use super::errors::{model_error_message, startup_notice};

pub struct AssistantTurnRequest {
    pub turn_id: u64,
    pub prompt: String,
    pub goal: Option<String>,
    pub history: Vec<ChatMessage>,
    pub session: ReplSession,
    pub cancel: CancellationToken,
    pub event_tx: broadcast::Sender<DashboardEvent>,
}

pub fn spawn_streaming_assistant_turn(request: AssistantTurnRequest) {
    tokio::spawn(async move {
        let AssistantTurnRequest {
            turn_id,
            prompt,
            goal,
            history,
            session,
            cancel,
            event_tx,
        } = request;

        let provider_id = session.provider.clone();
        let model_id = normalize_request_model_id(&provider_id, &session.model);
        let config = session.config.clone();
        let _ = event_tx.send(DashboardEvent::AssistantTurnStarted {
            turn_id,
            prompt: prompt.clone(),
        });

        let Some(provider) = continuum_models::load_from_config(&config, &provider_id) else {
            let message = startup_notice(&provider_id, &config).unwrap_or_else(|| {
                format!("No API key is configured for provider '{provider_id}'.")
            });
            let _ = event_tx.send(DashboardEvent::AssistantTurnFailed {
                turn_id,
                error: message,
            });
            return;
        };

        let mut messages = Vec::new();
        messages.push(Message::new(
            "system",
            "You are Continuum, a helpful CLI coding assistant. Keep replies concise, practical, and terminal-friendly.",
        ));
        if let Some(goal) = goal.as_deref() {
            if !goal.trim().is_empty() {
                messages.push(Message::new("system", format!("Session goal: {goal}")));
            }
        }
        for msg in history
            .iter()
            .filter(|m| matches!(m.role, MessageRole::User | MessageRole::Assistant))
        {
            let role = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System | MessageRole::Tool => continue,
            };
            messages.push(Message::new(role, msg.content.clone()));
        }
        messages.push(Message::new("user", prompt.clone()));

        let request =
            CompletionRequest::new(continuum_core::ids::ModelId::new(&model_id), messages)
                .with_temperature(0.2)
                .with_max_tokens(1200);

        let mut stream = match complete_with_openrouter_fallback(
            &provider,
            &provider_id,
            &model_id,
            request,
            cancel.child_token(),
            &event_tx,
            turn_id,
        )
        .await
        {
            Ok(stream) => stream,
            Err(err) => {
                let message = model_error_message(&provider_id, &err, &config);
                let _ = event_tx.send(DashboardEvent::AssistantTurnFailed {
                    turn_id,
                    error: message,
                });
                return;
            }
        };

        use futures::StreamExt;
        while let Some(delta) = stream.next().await {
            match delta {
                Ok(delta) => {
                    if !delta.text.is_empty() {
                        let _ = event_tx.send(DashboardEvent::AssistantTurnDelta {
                            turn_id,
                            delta: delta.text,
                        });
                    }
                    if delta.done {
                        break;
                    }
                }
                Err(err) => {
                    let message = model_error_message(&provider_id, &err, &config);
                    let _ = event_tx.send(DashboardEvent::AssistantTurnFailed {
                        turn_id,
                        error: message,
                    });
                    return;
                }
            }
        }

        let _ = event_tx.send(DashboardEvent::AssistantTurnCompleted { turn_id });
    });
}

async fn complete_with_openrouter_fallback(
    provider: &std::sync::Arc<dyn continuum_core::model::ModelProvider>,
    provider_id: &str,
    model_id: &str,
    request: CompletionRequest,
    cancel: CancellationToken,
    event_tx: &broadcast::Sender<DashboardEvent>,
    _turn_id: u64,
) -> Result<continuum_core::model::CompletionStream, continuum_core::model::ModelError> {
    match provider
        .complete(&Cap::grant(), request.clone(), cancel.clone())
        .await
    {
        Ok(stream) => Ok(stream),
        Err(err) => {
            if let Some(fallback_model) = openrouter_fallback_model(provider_id, model_id, &err) {
                let note = format!(
                    "OpenRouter had no endpoints for {model_id}; retrying with {fallback_model}"
                );
                let _ = event_tx.send(DashboardEvent::Log {
                    level: "INFO".into(),
                    target: "executor".into(),
                    message: note.clone(),
                });

                let fallback_request = CompletionRequest::new(
                    continuum_core::ids::ModelId::new(&normalize_request_model_id(
                        provider_id,
                        &fallback_model,
                    )),
                    request.messages.clone(),
                )
                .with_temperature(request.temperature.unwrap_or(0.2))
                .with_max_tokens(request.max_tokens.unwrap_or(1200));

                match provider
                    .complete(&Cap::grant(), fallback_request, cancel)
                    .await
                {
                    Ok(stream) => {
                        let _ = event_tx.send(DashboardEvent::ModelResolved {
                            provider: provider_id.to_string(),
                            model: fallback_model,
                            note: Some(note),
                        });
                        Ok(stream)
                    }
                    Err(err) => Err(err),
                }
            } else {
                Err(err)
            }
        }
    }
}

fn normalize_request_model_id(provider_id: &str, model_id: &str) -> String {
    if provider_id != "openrouter" {
        return model_id.to_string();
    }

    let mut suffix = model_id;
    while let Some(rest) = suffix.strip_prefix("openrouter/") {
        suffix = rest;
    }

    format!("openrouter/{suffix}")
}

#[cfg(test)]
mod tests {
    use super::normalize_request_model_id;

    #[test]
    fn openrouter_request_model_is_collapsed_to_single_prefix() {
        assert_eq!(
            normalize_request_model_id("openrouter", "openrouter/owl-alpha"),
            "openrouter/owl-alpha"
        );
        assert_eq!(
            normalize_request_model_id("openrouter", "owl-alpha"),
            "openrouter/owl-alpha"
        );
        assert_eq!(
            normalize_request_model_id("openrouter", "openrouter/openrouter/owl-alpha"),
            "openrouter/owl-alpha"
        );
    }
}

fn openrouter_fallback_model(
    provider_id: &str,
    model_id: &str,
    err: &continuum_core::model::ModelError,
) -> Option<String> {
    if provider_id != "openrouter" || !model_id.ends_with(":free") {
        return None;
    }

    let should_retry = match err {
        continuum_core::model::ModelError::ServerError { status, body, .. } => {
            *status == 404 && body.contains("No endpoints found")
        }
        continuum_core::model::ModelError::Other(message) => {
            message.contains("404") && message.contains("No endpoints found")
        }
        _ => false,
    };

    if should_retry {
        Some(model_id.trim_end_matches(":free").to_string())
    } else {
        None
    }
}
