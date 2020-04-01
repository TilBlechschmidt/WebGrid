// #[derive(Deserialize, Debug, Default)]
// struct CapabilityTimeouts {
//     script: Option<u32>,
//     pageLoad: Option<u32>,
//     implicit: Option<u32>
// }

// #[derive(Deserialize, Debug)]
// struct Capabilities {
//     acceptInsecureCerts: Option<bool>,
//     browserName: Option<String>,
//     browserVersion: Option<String>,
//     platformName: Option<String>,
//     pageLoadStrategy: Option<String>,
//     proxy: Option<String>,
//     strictFileInteractability: Option<bool>,
//     timeouts: Option<CapabilityTimeouts>,
//     unhandledPromptBehavior: Option<String>,

//     #[serde(flatten)]
//     extensionCapabilities: HashMap<String, Value>
// }