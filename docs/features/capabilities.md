# Extensions & Metadata

While WebGrid supports all standard capabilities, there is a number of extensions. These allow you to control features like screen recording and idle timeouts. Additionally, they can be used to attach metadata to a session. Later on, this metadata can be used to search for your session!

To get started, you have to create a map which contains all WebGrid specific settings. This map will later on be stored in the `webgrid:options` key of the capability object. The way to do this differs for each selenium library but below are a few examples to get you started.

=== "Java"
    ```java
    DesiredCapabilities desiredCapabilities = new DesiredCapabilities();
    final Map<String, Object> webgridOptions = new HashMap<>();
    
    // Put anything in your options map! See below for a list of whats available.
    
    desiredCapabilities.setCapability("webgrid:options", webgridOptions);
    ```

=== "Rust"
    ```rust
    let mut caps = DesiredCapabilities::firefox();
    
    // Put anything in your options map! See below for a list of whats available.
    caps.add_subkey("webgrid:options", /* Option key */, /* Option value */)?;
    
    WebDriver::new(endpoint, &caps).await?
    ```

## Disabling screen recording

We have optimized the heck out of screen recordings! They use almost no bandwidth and minimal CPU. For this reason, they are enabled globally if you have configured a storage backend. However, should you for some reason decide that you do not want to record a session, you can set the `disableRecording` flag in the `webgrid:options` capabilities.

=== "Java"
    ```java
    webgridOptions.put("disableRecording", true);
    ```

=== "Rust"
    ```rust
    caps.add_subkey("webgrid:options", "disableRecording", true);
    ```

!!! tip "Globally disable recordings"
    If you do not want recordings for any sessions, just do not configure a storage backend. Refer the corresponding installation guide on how to not do so!

## Overwriting idle timeout

To conserve resources, each session terminates automatically when it does not receive a command from a client within a certain time period. This is especially useful in scenarios where the client may have crashed. Since the protocol is not connection oriented, there is no other way to detect such a situation. The default timeout is set to about 10 minutes. When setting up the grid, you have to opportunity to set a different global default. However, maybe just some of your clients need to stay idle for a long time while others do not. For such situations, you can overwrite the idle timeout on a per-session basis by setting the `idleTimeout` key in the `webgrid:options` capabilities to any numeric value in seconds.

=== "Java"
    ```java
    webgridOptions.put("idleTimeout", 3600);
    ```

=== "Rust"
    ```rust
    caps.add_subkey("webgrid:options", "idleTimeout", 3600);
    ```

!!! warning "Very long timeouts"
    Setting a very long timeout may cause issues. As it is virtually impossible for the grid to detect a client that has crashed or otherwise disconnected in a non-clean fashion, such a sessions may become "orphaned" and stick around blocking resources for the timeout you set.

## Attaching metadata

When you run hundreds of sessions on the grid and do not have a way to store session identifiers, it can become hard to identify that one session after the fact — or maybe you want to run statistics on how many sessions each project has created in the last week. To solve this, you can attach arbitrary key-value metadata to each session by passing a map to the `metadata` key in the `webgrid:options` capabilities.

=== "Java"
    ```java
    final Map<String, String> metadata = new HashMap<>();
    metadata.put("project", "tardis");
    metadata.put("pipeline", "#42");

    webgridOptions.put("metadata", metadata);
    ```

=== "Rust"
    ```rust
    let mut metadata = HashMap::new();
    metadata.insert("project", "tardis");
    metadata.insert("pipeline", "#42");

    caps.add_subkey("webgrid:options", "metadata", metadata);
    ```

### Modifying metadata at runtime

Setting metadata up-front is nice. However, in certain scenarios you might want to attach additional or modify existing metadata while the session is running. For this, an extension command is available at `/<session-id>/webgrid/metadata`. You can either make a POST request to this URL with a JSON object as the request body, or use your libraries support for extension commands if available. In the future, support for legacy libraries which do not support extension commands will be added (using cookies with special names).

=== "Rust"
    Below is an example implementation of a metadata modification extension command for the [`thirtyfour`](https://github.com/stevepryde/thirtyfour) Rust library.
    ```rust
    struct WebgridMetadataCommand {
        fields: HashMap<String, String>,
    }

    impl WebgridMetadataCommand {
        pub fn new() -> Self {
            Self {
                fields: HashMap::new(),
            }
        }

        pub fn with_field(key: String, value: String) -> Self {
            let mut instance = Self::new();
            instance.add(key, value);
            instance
        }

        pub fn add(&mut self, key: String, value: String) {
            self.fields.insert(key, value);
        }
    }

    impl ExtensionCommand for WebgridMetadataCommand {
        fn parameters_json(&self) -> Option<serde_json::Value> {
            serde_json::to_value(self.fields.clone()).ok()
        }

        fn method(&self) -> thirtyfour::RequestMethod {
            thirtyfour::RequestMethod::Post
        }

        fn endpoint(&self) -> String {
            "/webgrid/metadata".into()
        }
    }

    // Usage
    let metadata_command = WebgridMetadataCommand::with_field("answer".into(), "42".into());
    driver.extension_command(metadata_command).await.ok();
    ```