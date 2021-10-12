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

    SessionId session = ((RemoteWebDriver) driver).getSessionId();
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
      session {
        latest {
        id
        metadata {
            client {
            key
            value
            }
        }
        }
      }
    }
    ```

    It returns you a list of all sessions with their corresponding identifiers and user-assigned metadata. To identify your session uniquely using metadata, head over to the [API documentation](./api.md#session-metadata).

## Embedding

You can embed the browser recording directly into your existing tools by using the JavaScript SDK. You need to import the JavaScript module and bind it to a DOM element of your choice. All CSS used is scoped to the `webgrid` class so it should not affect your website as long as you don't use this class in your styles.

```html
<body>
    <div id="<your-identifier>"></div>

    <script type="module">
        import { WebGridVideo } from 'http://<your-webgrid-address>/embed';

        new WebGridVideo({
            target: document.getElementById("<your-identifier>"),
            props: {
                sessionID: '<your-session-id>',
            }
        });
    </script>
</body>
```

By default, the script tries to guess the webgrid address from the import URL. This behaviour can be overruled by passing the `host: '<your-webgrid-address>'` property in the `props` object. Especially when fetching the script from within a static page builder which embeds it directly, this is required as the host is evaluated at runtime not at request time.

## Viewing

If you want to monitor your session manually you may use the dashboard provided by the grid. To do so just visit it at `http://<your-webgrid-address>` (without any path) and enter the previously obtained session ID.

!!! danger
    This dashboard is not supposed to be embedded into other pages through `iframes` or other means. For embedding refer the [embedding](#embedding) section!