# Getting started

Below are guides to get you started as quickly as possible on your specific platform! They provide a set of sane defaults which can later be tweaked for your use-case. 

## Docker

If you want to run a grid locally for simple testing purposes that require multiple isolated browsers or just want to evaluate this tool this is the right choice.
Make sure you have [Docker](https://www.docker.com/get-started) installed and configured properly.

Before you can start the grid in Docker you have to create a network for it:

```bash
docker network create webgrid
```

Next you need to download the latest `docker-compose.yml` and run it:

```bash
# Download compose file
curl -fsSLO https://webgrid.dev/docker-compose.yml

# Launch the grid
docker-compose up
```

Continue [reading below](#using-the-grid) on how to send requests to your grid.

## Kubernetes

WebGrid provides a [Helm](https://helm.sh) chart to get started as quickly as possible. Below is a guide on how to add the chart repository and install the chart.
You can change the name of the release in the second command or add other options like the target namespace — for more details consult the Helm documentation.

```bash
# Add the repository
helm repo add webgrid https://webgrid.dev/

# List all available versions
helm search repo --versions --devel webgrid/demo

# Install the chart
helm install example webgrid/demo --version "<pick-a-version-from-the-list>"
```

## Using the grid

Once you have started the grid you can send requests to it using the regular Selenium client libraries [available here](https://www.selenium.dev/documentation/en/).

=== "Java"
    ```java
    FirefoxOptions firefoxOptions = new FirefoxOptions();
    WebDriver driver = new RemoteWebDriver(new URL("http://localhost:8080"), firefoxOptions);
    driver.get("http://www.google.com");
    driver.quit();
    ```

=== "Python"
    ```python
    from selenium import webdriver

    firefox_options = webdriver.FirefoxOptions()
    driver = webdriver.Remote(
        command_executor='http://localhost:8080',
        options=firefox_options
    )
    driver.get("http://www.google.com")
    driver.quit() 
    ```

=== "C#"
    ```csharp
    FirefoxOptions firefoxOptions = new FirefoxOptions();
    IWebDriver driver = new RemoteWebDriver(new Uri("http://localhost:8080"), firefoxOptions);
    driver.Navigate().GoToUrl("http://www.google.com");
    driver.Quit();
    ```

=== "Ruby"
    ```ruby
    require 'selenium-webdriver'

    driver = Selenium::WebDriver.for :remote, url: "http://localhost:8080", desired_capabilities: :firefox
    driver.get "http://www.google.com"
    driver.close
    ```

=== "JavaScript"
    ```javascript
    const { Builder, Capabilities } = require("selenium-webdriver");
    var capabilities = Capabilities.firefox();
    (async function helloSelenium() {
        let driver = new Builder()
            .usingServer("http://localhost:8080")   
            .withCapabilities(capabilities)
            .build();
        try {
            await driver.get('http://www.google.com');
        } finally {
            await driver.quit();
        }
    })();
    ```

=== "Kotlin"
    ```kotlin
    firefoxOptions = FirefoxOptions()
    driver: WebDriver = new RemoteWebDriver(new URL("http://localhost:8080"), firefoxOptions)
    driver.get("http://www.google.com")
    driver.quit()
    ```

!!! attention
    When you used Kubernetes you may have to forward the grid service to your local computer for the example code to work. For details on accessing your WebGrid within a cluster consult the [Kubernetes specific](kubernetes/access.md) docs.