# Configuration

## Environment variables
Most parameters are passed to services through the environment. Below is a list of all available environment variables for each service.

### Proxy

| Variable | Description |
|:--|:--|
| `WEBGRID_REDIS_URL` | URL string used to connect to the database |

### Metrics

| Variable | Description |
|:--|:--|
| `WEBGRID_REDIS_URL` | URL string used to connect to the database |

### Manager

| Variable | Description |
|:--|:--|
| `WEBGRID_REDIS_URL` | URL string used to connect to the database |
| `WEBGRID_MANAGER_ID` | Identifier of this manager — must be unique |
| `WEBGRID_MANAGER_HOST` | Hostname under which this manager is reachable by other services |

### Node

| Variable | Description |
|:--|:--|
| `WEBGRID_REDIS_URL` | URL string used to connect to the database |
| `WEBGRID_DRIVER` | Path to the WebDriver executable |
| `WEBGRID_DRIVER_PORT` | Default port of the driver* |
| `WEBGRID_SESSION_ID` | Identifier of this node/session — must be unique |
| `WEBGRID_ON_SESSION_CREATE` | Path to executable that will be executed after the browser has been started |
| `BROWSER` | Type of browser — used internally for browser specific bug workarounds |

*Due to architectural constraints the application does not explicitly set the driver port through parameters. For this reason it is necessary to pass the default port it is listening on when started without parameters.


### Orchestrators

| Variable | Description |
|:--|:--|
| `WEBGRID_REDIS_URL` | URL string used to connect to the database |
| `WEBGRID_ORCHESTRATOR_ID` | Identifier of this orchestrator — must be unique |
| `WEBGRID_SLOTS` | How many parallel sessions this orchestrator may schedule |
| `WEBGRID_IMAGES` | List of images to use |

Images must be supplied in format `<image>:<tag>=<browser-name>::<browser-version>` where multiple images are separated by a comma. Example:

```
webgrid-node-firefox=firefox::68.7.0esr,webgrid-node-chrome=chrome::81.0.4044.122
```

#### Kubernetes
In addition to the above variables, there is a set of special variables that can be used in conjunction with the K8s orchestrator sub-type.

| Variable | Description |
|:--|:--|
| `NAMESPACE` | Namespace in which to deploy nodes, usually set to the current namespace by K8s (default: `webgrid`) |
| `WEBGRID_RESOURCE_PREFIX` | Name prefix that is applied to created resources |