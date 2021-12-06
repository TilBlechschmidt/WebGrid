(function() {var implementors = {};
implementors["domain"] = [{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"enum\" href=\"domain/enum.WebgridServiceDescriptor.html\" title=\"enum domain::WebgridServiceDescriptor\">WebgridServiceDescriptor</a>","synthetic":false,"types":["domain::discovery::WebgridServiceDescriptor"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/struct.SessionMetadata.html\" title=\"struct domain::SessionMetadata\">SessionMetadata</a>","synthetic":false,"types":["domain::session::SessionMetadata"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/event/struct.ProvisioningJobAssignedNotification.html\" title=\"struct domain::event::ProvisioningJobAssignedNotification\">ProvisioningJobAssignedNotification</a>","synthetic":false,"types":["domain::event::provisioner::ProvisioningJobAssignedNotification"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/event/struct.SessionCreatedNotification.html\" title=\"struct domain::event::SessionCreatedNotification\">SessionCreatedNotification</a>","synthetic":false,"types":["domain::event::session::created::SessionCreatedNotification"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/event/struct.SessionMetadataModifiedNotification.html\" title=\"struct domain::event::SessionMetadataModifiedNotification\">SessionMetadataModifiedNotification</a>","synthetic":false,"types":["domain::event::session::metadata::SessionMetadataModifiedNotification"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/event/struct.SessionOperationalNotification.html\" title=\"struct domain::event::SessionOperationalNotification\">SessionOperationalNotification</a>","synthetic":false,"types":["domain::event::session::operational::SessionOperationalNotification"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/event/struct.SessionProvisionedNotification.html\" title=\"struct domain::event::SessionProvisionedNotification\">SessionProvisionedNotification</a>","synthetic":false,"types":["domain::event::session::provisioned::SessionProvisionedNotification"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/event/struct.SessionScheduledNotification.html\" title=\"struct domain::event::SessionScheduledNotification\">SessionScheduledNotification</a>","synthetic":false,"types":["domain::event::session::scheduled::SessionScheduledNotification"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"enum\" href=\"domain/event/enum.SessionTerminationReason.html\" title=\"enum domain::event::SessionTerminationReason\">SessionTerminationReason</a>","synthetic":false,"types":["domain::event::session::terminated::SessionTerminationReason"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/event/struct.SessionTerminatedNotification.html\" title=\"struct domain::event::SessionTerminatedNotification\">SessionTerminatedNotification</a>","synthetic":false,"types":["domain::event::session::terminated::SessionTerminatedNotification"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/request/struct.ProvisionerMatchRequest.html\" title=\"struct domain::request::ProvisionerMatchRequest\">ProvisionerMatchRequest</a>","synthetic":false,"types":["domain::request::provisioner::ProvisionerMatchRequest"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/request/struct.ProvisionerMatchResponse.html\" title=\"struct domain::request::ProvisionerMatchResponse\">ProvisionerMatchResponse</a>","synthetic":false,"types":["domain::request::provisioner::ProvisionerMatchResponse"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/webdriver/struct.CapabilityTimeouts.html\" title=\"struct domain::webdriver::CapabilityTimeouts\">CapabilityTimeouts</a>","synthetic":false,"types":["domain::webdriver::capabilities::CapabilityTimeouts"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"enum\" href=\"domain/webdriver/enum.CapabilityPageLoadStrategy.html\" title=\"enum domain::webdriver::CapabilityPageLoadStrategy\">CapabilityPageLoadStrategy</a>","synthetic":false,"types":["domain::webdriver::capabilities::CapabilityPageLoadStrategy"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"enum\" href=\"domain/webdriver/enum.CapabilityUnhandledPromptBehavior.html\" title=\"enum domain::webdriver::CapabilityUnhandledPromptBehavior\">CapabilityUnhandledPromptBehavior</a>","synthetic":false,"types":["domain::webdriver::capabilities::CapabilityUnhandledPromptBehavior"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/webdriver/struct.CapabilitiesProxy.html\" title=\"struct domain::webdriver::CapabilitiesProxy\">CapabilitiesProxy</a>","synthetic":false,"types":["domain::webdriver::capabilities::CapabilitiesProxy"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/webdriver/struct.WebGridOptions.html\" title=\"struct domain::webdriver::WebGridOptions\">WebGridOptions</a>","synthetic":false,"types":["domain::webdriver::capabilities::WebGridOptions"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/webdriver/struct.Capabilities.html\" title=\"struct domain::webdriver::Capabilities\">Capabilities</a>","synthetic":false,"types":["domain::webdriver::capabilities::Capabilities"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/webdriver/struct.CapabilitiesRequest.html\" title=\"struct domain::webdriver::CapabilitiesRequest\">CapabilitiesRequest</a>","synthetic":false,"types":["domain::webdriver::capabilities::CapabilitiesRequest"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/webdriver/struct.RawCapabilitiesRequest.html\" title=\"struct domain::webdriver::RawCapabilitiesRequest\">RawCapabilitiesRequest</a>","synthetic":false,"types":["domain::webdriver::capabilities::RawCapabilitiesRequest"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/webdriver/struct.SessionCreateResponseValue.html\" title=\"struct domain::webdriver::SessionCreateResponseValue\">SessionCreateResponseValue</a>","synthetic":false,"types":["domain::webdriver::creation::SessionCreateResponseValue"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/webdriver/struct.SessionCreateResponse.html\" title=\"struct domain::webdriver::SessionCreateResponse\">SessionCreateResponse</a>","synthetic":false,"types":["domain::webdriver::creation::SessionCreateResponse"]},{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"domain/webdriver/struct.WebdriverError.html\" title=\"struct domain::webdriver::WebdriverError\">WebdriverError</a>","synthetic":false,"types":["domain::webdriver::error::WebdriverError"]}];
implementors["library"] = [{"text":"impl <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"library/communication/struct.BlackboxError.html\" title=\"struct library::communication::BlackboxError\">BlackboxError</a>","synthetic":false,"types":["library::communication::error::BlackboxError"]},{"text":"impl&lt;D:&nbsp;<a class=\"trait\" href=\"library/communication/discovery/trait.ServiceDescriptor.html\" title=\"trait library::communication::discovery::ServiceDescriptor\">ServiceDescriptor</a>&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"library/communication/discovery/struct.ServiceAnnouncement.html\" title=\"struct library::communication::discovery::ServiceAnnouncement\">ServiceAnnouncement</a>&lt;D&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;D: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,&nbsp;</span>","synthetic":false,"types":["library::communication::discovery::ServiceAnnouncement"]},{"text":"impl&lt;T&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a> for <a class=\"struct\" href=\"library/communication/event/struct.NotificationFrame.html\" title=\"struct library::communication::event::NotificationFrame\">NotificationFrame</a>&lt;T&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;T: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.130/serde/ser/trait.Serialize.html\" title=\"trait serde::ser::Serialize\">Serialize</a>,&nbsp;</span>","synthetic":false,"types":["library::communication::event::notification::NotificationFrame"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()