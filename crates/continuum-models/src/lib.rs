#![warn(missing_docs)]

//! Model provider implementations and routing.
//!
//! Each provider is feature-gated so you only compile what you need.
//! The `compat` module provides a generic OpenAI-compatible provider for
//! DeepSeek, Groq, Mistral, Together, Fireworks, and similar services.

#[cfg(feature = "anthropic")]
mod anthropic;
mod compat;
mod cost;
#[cfg(feature = "gemini")]
mod gemini;
#[cfg(feature = "ollama")]
mod ollama;
#[cfg(feature = "openai")]
mod openai;
mod router;
mod sse;

#[cfg(feature = "anthropic")]
pub use anthropic::AnthropicProvider;
pub use compat::CompatProvider;
#[cfg(feature = "gemini")]
pub use gemini::GeminiProvider;
#[cfg(feature = "ollama")]
pub use ollama::OllamaProvider;
#[cfg(feature = "openai")]
pub use openai::OpenAIProvider;
pub use router::CostAwareRouter;
