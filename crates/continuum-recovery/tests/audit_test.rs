use continuum_recovery::{AuditEvent, AuditLog, MemAuditLog};

#[tokio::test]
async fn test_mem_audit_record_and_query() {
    let log = MemAuditLog::new();
    let event = AuditEvent {
        event_type: "security_scan".into(),
        agent: "SecurityAgent".into(),
        task_id: Some("task-1".into()),
        detail: serde_json::json!({"findings": 3}),
        created_at: "2026-01-01T00:00:00Z".into(),
    };
    log.record(event).await.unwrap();

    let results = log.query("security_scan", 10).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].agent, "SecurityAgent");
}

#[tokio::test]
async fn test_audit_empty_query() {
    let log = MemAuditLog::new();
    let results = log.query("nonexistent", 10).await.unwrap();
    assert!(results.is_empty());
}

#[tokio::test]
async fn test_audit_limit() {
    let log = MemAuditLog::new();
    for i in 0..10 {
        let event = AuditEvent {
            event_type: "test".into(),
            agent: format!("agent-{}", i),
            task_id: None,
            detail: serde_json::json!({}),
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        log.record(event).await.unwrap();
    }
    let results = log.query("test", 3).await.unwrap();
    assert_eq!(results.len(), 3);
}
