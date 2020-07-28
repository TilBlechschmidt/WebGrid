#[macro_export]
macro_rules! unique_identifier {
    () => {
        concat!(module_path!(), file!(), line!(), column!())
    };
}

#[macro_export]
macro_rules! with_resource_manager {
    ($i:ident, $body:block) => {
        let mut runtime = tokio::runtime::Builder::new()
            .basic_scheduler()
            .threaded_scheduler()
            .enable_all()
            .build()
            .unwrap();

        let $i = $crate::libraries::resources::TEST_RESOURCE_PROVIDER.bind_resource_manager();

        // Pre-test resource setup
        runtime.block_on(async {
            use ::resources::ResourceManager;
            use ::scheduling::TaskResourceHandle;

            let mut redis = $i
                .redis(TaskResourceHandle::stub())
                .await
                .expect("Unable to get redis in test setup");

            redis.set_logging(false);
            $crate::setup::pre_test(&mut redis).await;
        });

        // Execute actual test
        let result = std::panic::catch_unwind(|| {
            let mut runtime = tokio::runtime::Builder::new()
                .basic_scheduler()
                .threaded_scheduler()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async { $body });
        });

        // Post-test resource cleanup
        runtime.block_on(async {
            use crate::libraries::resources::ResourceManager;
            use crate::libraries::scheduling::TaskResourceHandle;

            let mut redis = $i
                .redis(TaskResourceHandle::stub())
                .await
                .expect("Unable to get redis in test setup");

            redis.set_logging(false);
            $crate::setup::post_test(&mut redis).await;
        });

        // Unwind if the test failed
        if let Err(err) = result {
            std::panic::resume_unwind(err);
        }
    };
}
