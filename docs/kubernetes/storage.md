# Grid storage

Some features, like screen recording, require a persistent storage. Due to the vast possibilities to manage storage, no standard values are given and it is disabled by default. To enable it, merge and apply the following helm values as described [here](./configuration.md#changing-the-defaults):

```yaml
recording:
  enabled: true
```

??? tip "Limiting the storage size"
    By default the grid uses at most 50GB of storage for each volume and deletes old videos to keep the occupancy below that. You may change this value by adding the `recording.sizeLimit` value with an integer like `50` (representing GB). For more details, refer the [configuration defaults](./configuration.md#value-reference).

## Persistent Volume

To store the files a persistent volume is required. Due to the architecture of the grid the volume has to be mountable into multiple pods simultaneously! One way of achieving this is to allocate some storage on each worker node, however you may use any method your cluster allows.

### Local storage

The simplest method is a directory on each worker node used for the grid. Here is an example of a PersistentVolume:

```yaml
apiVersion: v1
kind: PersistentVolume
metadata:
  name: webgrid-pv
spec:
  capacity:
    storage: 50Gi
  volumeMode: Filesystem
  accessModes:
  - ReadWriteOnce
  persistentVolumeReclaimPolicy: Delete
  storageClassName: local-storage
  local:
    path: <storage-path-on-your-nodes>
```

You may want to add a node affinity to constrain it to nodes you plan on using with webgrid. To finish the setup, set the storage class as described [below](#volume-claim) to `local-storage`.

??? bug "Kubernetes bug workaround"
    During development, we encountered a bug which prevents a reuse of the PersistentVolume. When binding to the PV once and then re-binding later with another volume claim, the volume enters a blocked state which can only be resolved manually. To work around this, the chart includes the possibility to re-use an existing PVC for all tasks instead of creating its own.

    To enable this feature, manually create a volume claim like this:
    
    ```yaml
    apiVersion: v1
    kind: PersistentVolumeClaim
    metadata:
      name: <your-pvc-name>
    spec:
      storageClassName: local-storage
      accessModes:
      - ReadWriteOnce
      resources:
        requests:
          storage: 50Gi
    ```
    
    Then, merge and apply the following helm values as described [here](./configuration.md#changing-the-defaults)
    
    ```yaml
    recording:
      persistentVolumeClaim:
        create: false
        name: "<your-pvc-name>"
    ```

## Volume claim

By default, the helm chart creates its own PersistentVolumeClaim. To set the storage class, merge and apply the following helm values as described [here](./configuration.md#changing-the-defaults):

```yaml
recording:
  persistentVolumeClaim:
    storageClassName: "<your-storage-class>"
```