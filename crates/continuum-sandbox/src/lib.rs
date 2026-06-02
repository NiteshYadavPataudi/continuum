mod docker;

pub use docker::{DockerHandle, DockerSandbox};

#[cfg(test)]
mod tests {
    use continuum_core::sandbox::ExecRequest;

    #[test]
    fn test_sandbox_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<crate::DockerSandbox>();
    }

    #[test]
    fn test_exec_request_new() {
        let req = ExecRequest::new(vec!["bash".into(), "-c".into(), "echo hello".into()]);
        assert_eq!(req.argv.len(), 3);
        assert_eq!(req.argv[0], "bash");
        assert!(req.cwd.is_none());
        assert!(!req.stdin_open);
    }

    #[test]
    fn test_exec_request_with_cwd() {
        let mut req = ExecRequest::new(vec!["ls".into()]);
        req.cwd = Some("/workspace".into());
        assert_eq!(req.cwd.unwrap().to_str().unwrap(), "/workspace");
    }

    #[test]
    fn test_exec_request_with_env() {
        let mut req = ExecRequest::new(vec!["env".into()]);
        req.env.push(("MY_VAR".into(), "value".into()));
        assert_eq!(req.env.len(), 1);
        assert_eq!(req.env[0].0, "MY_VAR");
        assert_eq!(req.env[0].1, "value");
    }
}
