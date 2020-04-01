# Notes

## Node image
Browsers may crash frequently if not provided with enough memory. Docker by default allocates a shared host memory of 64MB which is not sufficient and may cause frequent browser crashes. This can be circumvented by using the `--shm-size=2g` option (in this case with an arbitrary option of 2G, which has been empirically proven to work well).

## Keyspace events
In order for the proxy server to get notified about managers, orchestrators and nodes coming up/going down key-space notifications need to be enabled. More specifically the string `Kgx` is required which can be set with `CONFIG SET notify-keyspace-events Kgx` at runtime or in the config file.