#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

pub struct VisionDoc {
    pub frontmatter: Option<VisionFrontmatter>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionFrontmatter {
    pub status: Option<String>,
    pub updated: Option<String>,
}

pub struct ProductDoc {
    pub frontmatter: Option<ProductFrontmatter>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFrontmatter {
    pub status: Option<String>,
    pub updated: Option<String>,
}

pub struct ArchitectureDoc {
    pub frontmatter: Option<ArchitectureFrontmatter>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureFrontmatter {
    pub status: Option<String>,
    pub updated: Option<String>,
}

pub struct EngineeringDoc {
    pub frontmatter: Option<EngineeringFrontmatter>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineeringFrontmatter {
    pub status: Option<String>,
    pub updated: Option<String>,
}

pub struct TasksDoc {
    pub frontmatter: Option<TasksFrontmatter>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TasksFrontmatter {
    pub status: Option<String>,
    pub updated: Option<String>,
}

pub struct AgentsDoc {
    pub frontmatter: Option<AgentsFrontmatter>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsFrontmatter {
    pub status: Option<String>,
    pub updated: Option<String>,
}

pub struct ModelRulesDoc {
    pub frontmatter: Option<ModelRulesFrontmatter>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRulesFrontmatter {
    pub status: Option<String>,
    pub updated: Option<String>,
}

pub struct SecurityDoc {
    pub frontmatter: Option<SecurityFrontmatter>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFrontmatter {
    pub status: Option<String>,
    pub updated: Option<String>,
}
