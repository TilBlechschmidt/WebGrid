# Screen recording

The grid is capable of capturing the screen for each browser session you create. This feature is enabled by default in a local Docker instance, but requires some additional configuration in a cluster.

!!! warning
    By default, this feature is disabled in the Kubernetes helm chart as it requires additional setup. Details on how to get up and running can be found on the [Kubernetes storage page](../kubernetes/storage.md)!

## Session ID

In order to view or embed a video you need to retrieve its unique session identifier. The simplest method is through your client library — below are a few examples:

=== "Java"
    ```java
    FirefoxOptions firefoxOptions = new FirefoxOptions();
    WebDriver driver = new RemoteWebDriver(new URL("http://localhost:8080"), firefoxOptions);
    driver.get("http://dcuk.com");

    SessionId session = ((FirefoxDriver) driver).getSessionId();
    System.out.println("Session id: " + session.toString());

    System.in.read();
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

    print("Session id: " + driver.session_id);

    input("Press Enter to quit...")
    driver.quit() 
    ```

??? tip "Retrieving session ID from API"
    Alternatively, you can get a list of all (or just active) sessions from the grid API. Note however that the list can be very large, so it might be hard to identify yours.

    Use the following query to get a list of sessions e.g. by pasting it into the [API explorer](./api.md) or using a GraphQL client:

    ```graphql
    query {
        sessions {
            id
            alive
        }
    }
    ```

    It returns you a list of all sessions with their corresponding identifiers and whether or not they are currently alive.

## Embedding

You can embed the browser recording directly into your existing tools by using the JavaScript SDK. You need to import the script in your `<head>` and add a custom HTML tag to the body where the video should be placed.

```html
<head>
    <script src="http://<your-webgrid-address>/embed" defer></script>
</head>
<body>
    <webgrid-video session-id="<your-session-id>"></webgrid-video>
</body>
```

By default, the script tries to guess the webgrid address from the script tag. This behaviour can be overruled by adding the `host="<your-webgrid-address"` attribute to the video tag. Especially when fetching the script for a static page builder which embeds it directly, this is required as the host is evaluated at runtime not at request time.

## Viewing

If you want to monitor your session manually you may use the video player provided by the grid. To do so just plug your session id from above into this url:

```
http://<your-webgrid-address>/embed/<session-id>
```

!!! danger
    This player is not supposed to be embedded into other pages through `iframes` or other means. For embedding refer the [embedding](#embedding) section!