use redis::{aio::ConnectionLike, RedisResult};

pub async fn pre_test<C: ConnectionLike>(con: &mut C) {
    let size: usize = redis::cmd("DBSIZE")
        .query_async(con)
        .await
        .expect("Failed to query database size before test");
    assert_eq!(size, 0, "Database is not empty! Maybe another test didn't clean up or you are attempting to test on a live database.");
}

pub async fn post_test<C: ConnectionLike>(con: &mut C) {
    redis::cmd("FLUSHDB")
        .query_async::<_, ()>(con)
        .await
        .expect("Failed to flush database after test!");
}

pub async fn load<C: ConnectionLike>(con: &mut C, content: &str) {
    for line in content.trim().lines() {
        let mut components = line.trim().split(' ');

        if let Some(command) = components.next() {
            let mut initial_command = redis::cmd(command);
            let mut command = &mut initial_command;

            for arg in components {
                command = command.arg(arg);
            }

            let result: RedisResult<()> = command.query_async(con).await;
            result.unwrap();
        }
    }
}
