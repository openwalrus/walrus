//! Cron scheduler tests.

use walrus_gateway::{CronJob, CronScheduler};

#[test]
fn parse_valid_cron_expression() {
    let job = CronJob::new(
        "test".into(),
        "0 0 9 * * *",
        "assistant".into(),
        "hello".to_string(),
    )
    .unwrap();
    assert_eq!(job.name.as_str(), "test");
    assert_eq!(job.agent.as_str(), "assistant");
    assert_eq!(job.message, "hello");
}

#[test]
fn invalid_cron_expression() {
    assert!(
        CronJob::new(
            "bad".into(),
            "not a cron",
            "assistant".into(),
            "hello".to_string(),
        )
        .is_err()
    );
}

#[test]
fn scheduler_from_jobs() {
    let jobs = vec![
        CronJob::new("job1".into(), "0 0 * * * *", "a".into(), "m1".to_string()).unwrap(),
        CronJob::new("job2".into(), "0 30 * * * *", "b".into(), "m2".to_string()).unwrap(),
    ];
    let scheduler = CronScheduler::new(jobs);
    assert_eq!(scheduler.jobs().len(), 2);
}

#[test]
fn scheduler_empty_jobs() {
    let scheduler = CronScheduler::new(vec![]);
    assert!(scheduler.jobs().is_empty());
}

#[test]
fn scheduler_invalid_expression_fails() {
    assert!(CronJob::new("bad".into(), "invalid", "a".into(), "m".to_string(),).is_err());
}
