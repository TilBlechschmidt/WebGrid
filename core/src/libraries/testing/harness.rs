#[cfg(test)]
#[macro_export]
macro_rules! unique_identifier {
    () => {
        concat!(module_path!(), file!(), line!(), column!())
    };
}

#[cfg(test)]
#[macro_export]
macro_rules! with_resource_manager {
    ($i:ident, $body:block) => {
        pretty_env_logger::formatted_builder()
            .filter_level(log::LevelFilter::max())
            .filter_module("mio", log::LevelFilter::Info)
            .is_test(true)
            .try_init()
            .ok();

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let $i =
            $crate::libraries::testing::resources::TEST_RESOURCE_PROVIDER.bind_resource_manager();

        let meta_resource_provider = $i.clone();

        // Pre-test resource setup
        runtime.block_on(async {
            use jatsl::TaskResourceHandle;
            use $crate::libraries::resources::ResourceManager;

            let mut redis = meta_resource_provider
                .redis(TaskResourceHandle::stub())
                .await
                .expect("Unable to get redis in test setup");

            redis.set_logging(false);
            $crate::libraries::testing::setup::pre_test(&mut redis).await;
        });

        // Execute actual test
        let result = std::panic::catch_unwind(|| {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async { $body });
        });

        // Post-test resource cleanup
        runtime.block_on(async {
            use crate::libraries::resources::ResourceManager;
            use jatsl::TaskResourceHandle;

            let mut redis = meta_resource_provider
                .redis(TaskResourceHandle::stub())
                .await
                .expect("Unable to get redis in test teardown");

            redis.set_logging(false);
            $crate::libraries::testing::setup::post_test(&mut redis).await;
        });

        // Unwind if the test failed
        if let Err(err) = result {
            std::panic::resume_unwind(err);
        }
    };
}
