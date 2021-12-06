(function() {var implementors = {};
implementors["domain"] = [{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"enum\" href=\"domain/webdriver/enum.BrowserParseError.html\" title=\"enum domain::webdriver::BrowserParseError\">BrowserParseError</a>&gt; for <a class=\"enum\" href=\"domain/container/enum.ContainerImageParseError.html\" title=\"enum domain::container::ContainerImageParseError\">ContainerImageParseError</a>","synthetic":false,"types":["domain::container::ContainerImageParseError"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"enum\" href=\"domain/event/enum.ModuleTerminationReason.html\" title=\"enum domain::event::ModuleTerminationReason\">ModuleTerminationReason</a>&gt; for <a class=\"enum\" href=\"domain/event/enum.SessionTerminationReason.html\" title=\"enum domain::event::SessionTerminationReason\">SessionTerminationReason</a>","synthetic":false,"types":["domain::event::session::terminated::SessionTerminationReason"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.56.1/std/primitive.tuple.html\">(</a><a class=\"enum\" href=\"domain/webdriver/enum.WebdriverErrorCode.html\" title=\"enum domain::webdriver::WebdriverErrorCode\">WebdriverErrorCode</a>, <a class=\"struct\" href=\"library/communication/error/struct.BlackboxError.html\" title=\"struct library::communication::error::BlackboxError\">BlackboxError</a><a class=\"primitive\" href=\"https://doc.rust-lang.org/1.56.1/std/primitive.tuple.html\">)</a>&gt; for <a class=\"struct\" href=\"domain/webdriver/struct.WebdriverError.html\" title=\"struct domain::webdriver::WebdriverError\">WebdriverError</a>","synthetic":false,"types":["domain::webdriver::error::WebdriverError"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.56.1/std/io/error/struct.Error.html\" title=\"struct std::io::error::Error\">Error</a>&gt; for <a class=\"enum\" href=\"domain/webdriver/enum.WebDriverError.html\" title=\"enum domain::webdriver::WebDriverError\">WebDriverError</a>","synthetic":false,"types":["domain::webdriver::instance::WebDriverError"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;Error&gt; for <a class=\"enum\" href=\"domain/webdriver/enum.WebDriverError.html\" title=\"enum domain::webdriver::WebDriverError\">WebDriverError</a>","synthetic":false,"types":["domain::webdriver::instance::WebDriverError"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://docs.rs/http/0.2.5/http/error/struct.Error.html\" title=\"struct http::error::Error\">Error</a>&gt; for <a class=\"enum\" href=\"domain/webdriver/enum.WebDriverError.html\" title=\"enum domain::webdriver::WebDriverError\">WebDriverError</a>","synthetic":false,"types":["domain::webdriver::instance::WebDriverError"]}];
implementors["library"] = [{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;&amp;'_ (dyn <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/std/error/trait.Error.html\" title=\"trait std::error::Error\">Error</a> + 'static)&gt; for <a class=\"struct\" href=\"library/communication/struct.BlackboxError.html\" title=\"struct library::communication::BlackboxError\">BlackboxError</a>","synthetic":false,"types":["library::communication::error::BlackboxError"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://docs.rs/http/0.2.5/http/header/value/struct.InvalidHeaderValue.html\" title=\"struct http::header::value::InvalidHeaderValue\">InvalidHeaderValue</a>&gt; for <a class=\"enum\" href=\"library/http/enum.ForwardError.html\" title=\"enum library::http::ForwardError\">ForwardError</a>","synthetic":false,"types":["library::http::forward::ForwardError"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://docs.rs/http/0.2.5/http/header/value/struct.ToStrError.html\" title=\"struct http::header::value::ToStrError\">ToStrError</a>&gt; for <a class=\"enum\" href=\"library/http/enum.ForwardError.html\" title=\"enum library::http::ForwardError\">ForwardError</a>","synthetic":false,"types":["library::http::forward::ForwardError"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;Error&gt; for <a class=\"enum\" href=\"library/http/enum.ForwardError.html\" title=\"enum library::http::ForwardError\">ForwardError</a>","synthetic":false,"types":["library::http::forward::ForwardError"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/1.56.1/core/convert/trait.From.html\" title=\"trait core::convert::From\">From</a>&lt;<a class=\"struct\" href=\"https://docs.rs/anyhow/1.0.45/anyhow/struct.Error.html\" title=\"struct anyhow::Error\">Error</a>&gt; for <a class=\"enum\" href=\"library/storage/s3/enum.S3StorageError.html\" title=\"enum library::storage::s3::S3StorageError\">S3StorageError</a>","synthetic":false,"types":["library::storage::s3::S3StorageError"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()