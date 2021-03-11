# API

The grid exposes an API which provides read-only access to the status and metadata of sessions and some other internal components which might be of interest. It uses the [GraphQL](https://graphql.org) query language to provide predictable and typed responses and is available under the following URL:

```
http://<your-webgrid-address>/api
```

!!! tip
    When opening the API in a browser, a GraphQL Playground opens up which provides syntax highlighting, code completion and a complete documentation!

## Session metadata

When creating a session, you can attach additional metadata which you can later use to query for your session. To do so, set one or both of the following keys in your client libraries `DesiredCapabilities` object.

=== "Java"
    ```java
    DesiredCapabilities desiredCapabilities = new DesiredCapabilities();
    ...
    final Map<String, String> webgridOptions = new HashMap<>();
    webgridOptions.put("name", "<your-test-name>");
    webgridOptions.put("build", "<your-build-id>");
    desiredCapabilities.setCapability("webgrid:options", webgridOptions);
    ...
    ```

To query session objects which match your name, use a query like this:

```javascript
query {
  sessions(name: "test-name") {
    id
    metadata {
      name
      build
    }
  }
}
```