# Grid storage

Some features, like screen recording, require a persistent storage. Due to the vast possibilities to manage storage, no standard values are given and it is disabled by default. To enable it, merge and apply the following helm values as described [here](./configuration.md#changing-the-defaults):

```yaml
config:
  storageBackend: <your-storage-url>
```

## S3 compatible

Currently, only S3 compatible storage providers are supported. You can use them by providing a URl that follows this schema:

```
s3+http(s)://user:pass@storageHost/bucket(?pathStyle)
```

Here is an example for usage with a Minio instance hosted on `webgrid-demo-storage` with user `webgrid`, password `supersecret`, and bucket `webgrid-video`. Note that Minio uses path-style bucket URLs by default instead of subdomain style ones, so the storage URL contains the corresponding suffix.

```
s3+http://webgrid:supersecret@webgrid-demo-storage/webgrid-video?pathStyle
```
