use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::Cap;
use continuum_core::sandbox::{ExecEvent, ExecRequest};
use continuum_core::validator::{
    Finding, Severity, ValidationContext, ValidationError, ValidationReport, ValidationStage,
    ValidationTarget, Validator,
};

macro_rules! lang_validator {
    ($name:ident, $stage:expr, $required:expr, $check_cmd:expr, $ext:expr) => {
        #[derive(Debug)]
        pub struct $name;

        #[async_trait]
        impl Validator for $name {
            fn stage(&self) -> ValidationStage { $stage }
            fn required(&self) -> bool { $required }

            async fn run(
                &self,
                target: &ValidationTarget,
                _ctx: &ValidationContext,
            ) -> Result<ValidationReport, ValidationError> {
                let start = std::time::Instant::now();
                let has_ext = target.paths.iter().any(|p| p.extension().map(|e| e == $ext).unwrap_or(false));
                if !has_ext {
                    return Ok(ValidationReport::new($stage, vec![], true, 0));
                }

                let handle = target.sandbox.as_ref()
                    .ok_or_else(|| ValidationError::Other("no sandbox".into()))?;
                let mut exec = ExecRequest::new($check_cmd);
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
                    vec![Finding::new(stringify!($name), Severity::Error, format!("{} failed (exit {})", stringify!($name), exit_code), None, None)]
                };
                Ok(ValidationReport::new($stage, findings, passed, duration))
            }
        }
    };
}

lang_validator!(BiomeValidator, ValidationStage::Lint, false, vec!["npx".into(), "biome".into(), "check".into(), ".".into()], "ts");
lang_validator!(VitestValidator, ValidationStage::UnitTest, false, vec!["npx".into(), "vitest".into(), "run".into()], "ts");
lang_validator!(PytestValidator, ValidationStage::UnitTest, false, vec!["python3".into(), "-m".into(), "pytest".into()], "py");
