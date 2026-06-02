use serde::{Deserialize, Serialize};

/// Parsed VISION.md document with optional frontmatter and body.
pub struct VisionDoc {
    /// Optional YAML frontmatter parsed from the document.
    pub frontmatter: Option<VisionFrontmatter>,
    /// The markdown body content.
    pub body: String,
}

/// YAML frontmatter for VISION.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionFrontmatter {
    /// Document status (e.g. "draft", "approved").
    pub status: Option<String>,
    /// ISO-8601 date of last update.
    pub updated: Option<String>,
}

/// Parsed PRODUCT.md document with optional frontmatter and body.
pub struct ProductDoc {
    /// Optional YAML frontmatter parsed from the document.
    pub frontmatter: Option<ProductFrontmatter>,
    /// The markdown body content.
    pub body: String,
}

/// YAML frontmatter for PRODUCT.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFrontmatter {
    /// Document status (e.g. "draft", "approved").
    pub status: Option<String>,
    /// ISO-8601 date of last update.
    pub updated: Option<String>,
}

/// Parsed ARCHITECTURE.md document with optional frontmatter and body.
pub struct ArchitectureDoc {
    /// Optional YAML frontmatter parsed from the document.
    pub frontmatter: Option<ArchitectureFrontmatter>,
    /// The markdown body content.
    pub body: String,
}

/// YAML frontmatter for ARCHITECTURE.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureFrontmatter {
    /// Document status (e.g. "draft", "approved").
    pub status: Option<String>,
    /// ISO-8601 date of last update.
    pub updated: Option<String>,
}

/// Parsed ENGINEERING.md document with optional frontmatter and body.
pub struct EngineeringDoc {
    /// Optional YAML frontmatter parsed from the document.
    pub frontmatter: Option<EngineeringFrontmatter>,
    /// The markdown body content.
    pub body: String,
}

/// YAML frontmatter for ENGINEERING.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineeringFrontmatter {
    /// Document status (e.g. "draft", "approved").
    pub status: Option<String>,
    /// ISO-8601 date of last update.
    pub updated: Option<String>,
}

/// Parsed TASKS.md document with optional frontmatter and body.
pub struct TasksDoc {
    /// Optional YAML frontmatter parsed from the document.
    pub frontmatter: Option<TasksFrontmatter>,
    /// The markdown body content.
    pub body: String,
}

/// YAML frontmatter for TASKS.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksFrontmatter {
    /// Document status (e.g. "draft", "approved").
    pub status: Option<String>,
    /// ISO-8601 date of last update.
    pub updated: Option<String>,
}

/// Parsed AGENTS.md document with optional frontmatter and body.
pub struct AgentsDoc {
    /// Optional YAML frontmatter parsed from the document.
    pub frontmatter: Option<AgentsFrontmatter>,
    /// The markdown body content.
    pub body: String,
}

/// YAML frontmatter for AGENTS.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsFrontmatter {
    /// Document status (e.g. "draft", "approved").
    pub status: Option<String>,
    /// ISO-8601 date of last update.
    pub updated: Option<String>,
}

/// Parsed MODEL_RULES.md document with optional frontmatter and body.
pub struct ModelRulesDoc {
    /// Optional YAML frontmatter parsed from the document.
    pub frontmatter: Option<ModelRulesFrontmatter>,
    /// The markdown body content.
    pub body: String,
}

/// YAML frontmatter for MODEL_RULES.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRulesFrontmatter {
    /// Document status (e.g. "draft", "approved").
    pub status: Option<String>,
    /// ISO-8601 date of last update.
    pub updated: Option<String>,
}

/// Parsed SECURITY.md document with optional frontmatter and body.
pub struct SecurityDoc {
    /// Optional YAML frontmatter parsed from the document.
    pub frontmatter: Option<SecurityFrontmatter>,
    /// The markdown body content.
    pub body: String,
}

/// YAML frontmatter for SECURITY.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFrontmatter {
    /// Document status (e.g. "draft", "approved").
    pub status: Option<String>,
    /// ISO-8601 date of last update.
    pub updated: Option<String>,
}
