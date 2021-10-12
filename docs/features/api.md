# API

The grid exposes an API which provides read-only access to the status and metadata of sessions and some other internal components which might be of interest. It uses the [GraphQL](https://graphql.org) query language to provide predictable and typed responses and is available under the following URL:

```
http://<your-webgrid-address>/api
```

!!! tip
    When opening the API in a browser, a GraphQL Playground opens up which provides syntax highlighting, code completion and a complete documentation!

## Session metadata

If you have attached metadata to your session as described [over here](./capabilities.md#attaching-metadata), you can search for your session by using regular expressions. To do so, you can use a query that looks like this:

```graphql
query {
  session {
    query(fields: [
      { key: "project", regex: "^tardis$" },
      { key: "answer", regex: "\\d+" },
    ]) {
      id
    }
  }
}
```

You can also fetch the latest sessions or retrieve details of a session given its identifier. For more details, consult the self-documenting API at `/api`.