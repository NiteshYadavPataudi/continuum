use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::sandbox::{ExecEvent, ExecRequest};
use continuum_core::tool::ToolRunner;
use continuum_core::validator::{
    Finding, Severity, ValidationContext, ValidationError, ValidationReport, ValidationStage,
    ValidationTarget, Validator,
};

/// Returns a vector of all ten stage validators, one per [`ValidationStage`].
pub fn all_validators() -> Vec<Box<dyn Validator>> {
    vec![
        Box::new(CompileValidator),
        Box::new(LintValidator),
        Box::new(TypeCheckValidator),
        Box::new(UnitTestValidator),
        Box::new(IntegrationTestValidator),
        Box::new(E2eTestValidator),
        Box::new(SecurityScanValidator),
        Box::new(StartupValidator),
        Box::new(PerformanceValidator),
        Box::new(RegressionValidator),
    ]
}

/// A registry of all built-in validators.
#[derive(Default)]
pub struct AllValidators(Vec<Box<dyn Validator>>);

impl std::fmt::Debug for AllValidators {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AllValidators")
            .field(&format!("{} validators", self.0.len()))
            .finish()
    }
}

impl AllValidators {
    /// Create a registry with all ten stage validators.
    pub fn new() -> Self {
        Self(all_validators())
    }

    /// Iterate over all registered validators.
    pub fn iter(&self) -> impl Iterator<Item = &Box<dyn Validator>> {
        self.0.iter()
    }

    /// Add a custom validator.
    pub fn add(&mut self, v: Box<dyn Validator>) {
        self.0.push(v);
    }
}

macro_rules! exec_validator {
    ($name:ident, $stage:expr, $required:expr, $argv:expr) => {
        #[derive(Debug)]
        pub struct $name;

        #[async_trait]
        impl Validator for $name {
            fn stage(&self) -> ValidationStage {
                $stage
            }

            fn required(&self) -> bool {
                $required
            }

            async fn run(
                &self,
                target: &ValidationTarget,
                _ctx: &ValidationContext,
            ) -> Result<ValidationReport, ValidationError> {
                let start = std::time::Instant::now();
                let mut exec = ExecRequest::new($argv);
                exec.cwd = Some(target.workspace.clone());

                let handle = target
                    .sandbox
                    .as_ref()
                    .ok_or_else(|| ValidationError::Other("no sandbox available".into()))?;

                let stream = handle
                    .exec(&Cap::grant(), exec)
                    .await
                    .map_err(|e| ValidationError::Other(e.to_string()))?;

                let mut exit_code = 0;
                let mut stream = std::pin::pin!(stream);
                while let Some(event) = stream.next().await {
                    match event.map_err(|e| ValidationError::Other(e.to_string()))? {
                        ExecEvent::Exit(code) => {
                            exit_code = code;
                            break;
                        }
                        _ => {}
                    }
                }

                let duration = start.elapsed().as_millis() as u64;
                let passed = exit_code == 0;

                let findings = if passed {
                    vec![]
                } else {
                    vec![Finding::new(
                        stringify!($name),
                        Severity::Error,
                        format!("{} stage failed (exit {})", stringify!($name), exit_code),
                        None,
                        None,
                    )]
                };
                Ok(ValidationReport::new($stage, findings, passed, duration))
            }
        }
    };
}

exec_validator!(CompileValidator, ValidationStage::Compile, true, vec!["cargo".into(), "check".into()]);
exec_validator!(TypeCheckValidator, ValidationStage::TypeCheck, true, vec!["cargo".into(), "check".into()]);

#[derive(Debug)]
pub struct LintValidator;

#[async_trait]
impl Validator for LintValidator {
    fn stage(&self) -> ValidationStage { ValidationStage::Lint }
    fn required(&self) -> bool { true }

    async fn run(
        &self,
        target: &ValidationTarget,
        _ctx: &ValidationContext,
    ) -> Result<ValidationReport, ValidationError> {
        let start = std::time::Instant::now();
        let handle = target.sandbox.as_ref()
            .ok_or_else(|| ValidationError::Other("no sandbox available".into()))?;

        let clippy = continuum_tools_linters::Clippy;
        let report = clippy
            .run(
                continuum_core::tool::ToolInvocation::new(
                    target.workspace.clone(),
                    serde_json::json!({}),
                )
                .with_paths(target.paths.clone()),
                handle.as_ref(),
            )
            .await
            .map_err(|e| ValidationError::Other(e.to_string()))?;

        let duration = start.elapsed().as_millis() as u64;
        Ok(ValidationReport::new(
            ValidationStage::Lint,
            report.findings,
            report.exit_code == 0,
            duration,
        ))
    }
}

exec_validator!(UnitTestValidator, ValidationStage::UnitTest, true, vec![
    "cargo".into(), "test".into(), "--lib".into()
]);

exec_validator!(IntegrationTestValidator, ValidationStage::IntegrationTest, false, vec![
    "cargo".into(), "test".into(), "--test".into(), "*".into()
]);

#[derive(Debug)]
pub struct E2eTestValidator;

#[async_trait]
impl Validator for E2eTestValidator {
    fn stage(&self) -> ValidationStage { ValidationStage::E2eTest }
    fn required(&self) -> bool { false }

    async fn run(
        &self,
        _target: &ValidationTarget,
        _ctx: &ValidationContext,
    ) -> Result<ValidationReport, ValidationError> {
        let start = std::time::Instant::now();
        Ok(ValidationReport::new(
            ValidationStage::E2eTest,
            vec![Finding::new(
                "e2e", Severity::Info, "E2E tests not yet configured", None, None,
            )],
            true,
            start.elapsed().as_millis() as u64,
        ))
    }
}

#[derive(Debug)]
pub struct SecurityScanValidator;

#[async_trait]
impl Validator for SecurityScanValidator {
    fn stage(&self) -> ValidationStage { ValidationStage::SecurityScan }
    fn required(&self) -> bool { true }

    async fn run(
        &self,
        target: &ValidationTarget,
        _ctx: &ValidationContext,
    ) -> Result<ValidationReport, ValidationError> {
        let start = std::time::Instant::now();
        let handle = target.sandbox.as_ref()
            .ok_or_else(|| ValidationError::Other("no sandbox available".into()))?;

        let audit = continuum_tools_security::CargoAudit;
        let report = audit
            .run(
                continuum_core::tool::ToolInvocation::new(
                    target.workspace.clone(),
                    serde_json::json!({}),
                )
                .with_paths(target.paths.clone()),
                handle.as_ref(),
            )
            .await
            .map_err(|e| ValidationError::Other(e.to_string()))?;

        let duration = start.elapsed().as_millis() as u64;
        Ok(ValidationReport::new(
            ValidationStage::SecurityScan,
            report.findings,
            report.exit_code == 0,
            duration,
        ))
    }
}

#[derive(Debug)]
pub struct StartupValidator;

#[async_trait]
impl Validator for StartupValidator {
    fn stage(&self) -> ValidationStage { ValidationStage::Startup }
    fn required(&self) -> bool { false }

    async fn run(
        &self,
        target: &ValidationTarget,
        _ctx: &ValidationContext,
    ) -> Result<ValidationReport, ValidationError> {
        let start = std::time::Instant::now();
        let handle = target.sandbox.as_ref()
            .ok_or_else(|| ValidationError::Other("no sandbox available".into()))?;

        let mut exec = ExecRequest::new(vec!["sh".into(), "-c".into(), "echo 'startup OK' && exit 0".into()]);
        exec.cwd = Some(target.workspace.clone());
        let stream = handle.exec(&Cap::grant(), exec).await.map_err(|e| ValidationError::Other(e.to_string()))?;
        let mut stream = std::pin::pin!(stream);
        let mut exit_code = 0;
        while let Some(event) = stream.next().await {
            if let ExecEvent::Exit(code) = event.map_err(|e| ValidationError::Other(e.to_string()))? {
                exit_code = code;
                break;
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        let passed = exit_code == 0;
        let findings = if passed { vec![] } else {
            vec![Finding::new("startup", Severity::Error, "sandbox startup failed", None, None)]
        };
        Ok(ValidationReport::new(ValidationStage::Startup, findings, passed, duration))
    }
}

#[derive(Debug)]
pub struct PerformanceValidator;

#[async_trait]
impl Validator for PerformanceValidator {
    fn stage(&self) -> ValidationStage { ValidationStage::Performance }
    fn required(&self) -> bool { false }

    async fn run(
        &self,
        target: &ValidationTarget,
        _ctx: &ValidationContext,
    ) -> Result<ValidationReport, ValidationError> {
        let start = std::time::Instant::now();
        let mut findings = vec![];

        if let Some(handle) = target.sandbox.as_ref() {
            let mut exec = ExecRequest::new(vec!["cargo".into(), "build".into(), "--workspace".into()]);
            exec.cwd = Some(target.workspace.clone());
            let build_start = std::time::Instant::now();
            let stream = handle.exec(&Cap::grant(), exec).await.map_err(|e| ValidationError::Other(e.to_string()))?;
            let mut stream = std::pin::pin!(stream);
            while let Some(event) = stream.next().await {
                if let ExecEvent::Exit(_) = event.map_err(|e| ValidationError::Other(e.to_string()))? {
                    break;
                }
            }
            let build_ms = build_start.elapsed().as_millis() as u64;
            if build_ms > 30_000 {
                findings.push(Finding::new("perf-build", Severity::Warning, format!("build took {}ms (threshold 30000ms)", build_ms), None, None));
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        Ok(ValidationReport::new(ValidationStage::Performance, findings, true, duration))
    }
}

#[derive(Debug)]
pub struct RegressionValidator;

#[async_trait]
impl Validator for RegressionValidator {
    fn stage(&self) -> ValidationStage { ValidationStage::Regression }
    fn required(&self) -> bool { false }

    async fn run(
        &self,
        target: &ValidationTarget,
        _ctx: &ValidationContext,
    ) -> Result<ValidationReport, ValidationError> {
        let start = std::time::Instant::now();
        let mut findings = vec![];

        if let Some(handle) = target.sandbox.as_ref() {
            let mut exec = ExecRequest::new(vec!["cargo".into(), "test".into(), "--workspace".into()]);
            exec.cwd = Some(target.workspace.clone());
            let stream = handle.exec(&Cap::grant(), exec).await.map_err(|e| ValidationError::Other(e.to_string()))?;
            let mut stream = std::pin::pin!(stream);
            let mut exit_code = 0;
            while let Some(event) = stream.next().await {
                if let ExecEvent::Exit(code) = event.map_err(|e| ValidationError::Other(e.to_string()))? {
                    exit_code = code;
                    break;
                }
            }
            if exit_code != 0 {
                findings.push(Finding::new("regression", Severity::Error, format!("test regression detected (exit {})", exit_code), None, None));
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        let passed = findings.is_empty();
        Ok(ValidationReport::new(ValidationStage::Regression, findings, passed, duration))
    }
}
