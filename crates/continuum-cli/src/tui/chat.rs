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
        let model_id = session.model.clone();
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

        let mut stream = match provider
            .complete(&Cap::grant(), request, cancel.child_token())
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
