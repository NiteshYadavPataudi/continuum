use async_trait::async_trait;
use futures::StreamExt;

use continuum_core::caps::{Cap, HostExec};
use continuum_core::ids::ToolId;
use continuum_core::sandbox::{ExecEvent, ExecRequest, SandboxHandle};
use continuum_core::tool::{ToolError, ToolFamily, ToolInvocation, ToolReport, ToolRunner};
use continuum_core::validator::Finding;

#[derive(Debug)]
pub struct ChromiumoxideRunner;

#[async_trait]
impl ToolRunner for ChromiumoxideRunner {
    fn id(&self) -> ToolId {
        ToolId::new("chromiumoxide")
    }

    fn family(&self) -> ToolFamily {
        ToolFamily::Browser
    }

    fn supports(&self, project: &continuum_core::tool::ProjectKind) -> bool {
        project.language == "javascript" || project.language == "typescript"
    }

    async fn run(
        &self,
        invocation: ToolInvocation,
        sandbox: &dyn SandboxHandle,
    ) -> Result<ToolReport, ToolError> {
        let start = std::time::Instant::now();

        let url = invocation.args.get("url").and_then(|v| v.as_str()).unwrap_or("about:blank");
        let script = invocation.args.get("script").and_then(|v| v.as_str()).unwrap_or("");

        let node_script = format!(
            r#"const puppeteer = require('puppeteer');
(async () => {{
    const browser = await puppeteer.launch({{ headless: true, args: ['--no-sandbox'] }});
    const page = await browser.newPage();
    await page.goto('{url}', {{ waitUntil: 'networkidle0' }});
    {script}
    await browser.close();
}})();"#
        );

        let mut exec = ExecRequest::new(vec!["node".into(), "-e".into(), node_script]);
        exec.cwd = Some(invocation.workspace.clone());

        let stream = sandbox
            .exec(&Cap::grant(), exec)
            .await
            .map_err(|e| ToolError::Sandbox(e.to_string()))?;

        let mut exit_code = 0;
        let mut stream = std::pin::pin!(stream);
        while let Some(event) = stream.next().await {
            match event.map_err(|e| ToolError::Sandbox(e.to_string()))? {
                ExecEvent::Exit(code) => {
                    exit_code = code;
                    break;
                }
                _ => {}
            }
        }

        let duration = start.elapsed().as_millis() as u64;
        let findings = if exit_code == 0 {
            vec![]
        } else {
            vec![Finding::new("chromiumoxide", continuum_core::validator::Severity::Error, "browser automation failed", None, None)]
        };
        Ok(ToolReport::new(self.id(), findings, exit_code, duration))
    }
}
