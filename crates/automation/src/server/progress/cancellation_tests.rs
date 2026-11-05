use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn dropping_a_waited_command_cancels_its_job() {
    let mcp = PhotocraftMcp::headless();
    mcp.headless_op(|headless| headless.command_run("file.new", json!({"width": 8, "height": 8}))).await.unwrap().unwrap();
    let before = mcp.headless_op(|headless| headless.inspect(None)).await.unwrap().unwrap();
    let backend = mcp.backend.clone();
    let (started, began) = tokio::sync::oneshot::channel();
    let guard = mcp
        .headless_op(move |headless| {
            let job = headless.session.start_job(
                "test.waitedCancellation",
                json!({}),
                "Waited cancellation",
                true,
                move |context| -> photocraft_engine::Result<()> {
                    let _ = started.send(());
                    loop {
                        context.check()?;
                        std::thread::sleep(Duration::from_millis(1));
                    }
                },
                |session, ()| session.execute("layer.new.layer", json!({"name": "Must not apply"})),
            )?;
            match job {
                Started::Job(job) => Ok(Abandon { backend, job: Some(job) }),
                Started::Done(_) => Err(AutomationError::Other("the gated test job did not start in the background".into())),
            }
        })
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(30), began).await.unwrap().unwrap();
    assert!(mcp.headless_op(|headless| Ok(headless.session.has_jobs())).await.unwrap().unwrap());
    let (waiting, polled) = tokio::sync::oneshot::channel();
    let mut future = Box::pin(async move {
        let _guard = guard;
        waiting.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    tokio::select! {
        result = polled => result.unwrap(),
        () = &mut future => panic!("the gated wait unexpectedly finished"),
    }
    drop(future);
    let deadline = Instant::now();
    loop {
        let (running, cancelled) = mcp
            .headless_op(|headless| Ok((headless.session.has_jobs(), headless.session.jobs_with_recent().iter().any(|job| job.state == "cancelled"))))
            .await
            .unwrap()
            .unwrap();
        if !running {
            assert!(cancelled, "the job ended without being cancelled");
            break;
        }
        assert!(deadline.elapsed() < Duration::from_secs(30), "the dropped job kept running");
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    assert_eq!(mcp.headless_op(|headless| headless.inspect(None)).await.unwrap().unwrap(), before);
}
