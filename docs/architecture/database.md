# Database

All metadata is stored in a key-value in-memory database called [Redis](https://redis.io).  Below is a list of all keys that are currently in use, annotated with their type and format (if applicable).

## Root lists
```javascript
`orchestrators` = Set<string>       // uuids
`sessions.active` = Set<string>     // uuids
`sessions.terminated` = Set<string> // uuids
```

## Configuration
```javascript
`timeouts` = Hashes {
	queue = number                  // seconds
	scheduling = number             // seconds
	nodeStartup = number            // seconds
	driverStartup = number          // seconds
	sessionTermination = number     // seconds
	slotReclaimInterval = number    // seconds
}
```

## Manager
```javascript
`manager:${ID}:host` = number EX 120s			// (host + port)
```

## Storage
```javascript
// SID = storage ID
// PID = randomly generated ephemeral provider ID
`storage:${SID}:${PID}:host` = string EX 60s	// (host + port)
```

## API
```javascript
// AID = randomly generated ephemeral server ID
`api:${AID}:host` = string EX 60s				// (host + port)
```

## Sessions
```javascript
// ID = unique, external session identifier
`session:${ID}:heartbeat.node` = string EX 60s      // RFC 3339
`session:${ID}:heartbeat.manager` = string EX 30s   // RFC 3339

`session:${ID}:slot` = string                       // slot ID
`session:${ID}:orchestrator` = List<string>         // orchestrator ID

`session:${ID}:log` = Stream {
	component = 'node' | 'orchestrator' | 'manager' | 'proxy'
	level = 'info' | 'warn' | 'fail'
	code = string                                   // event type, see below
	meta = string                                   // additional information
}

`session:${ID}:status` = Hashes {
	queuedAt = string                               // RFC 3339
	pendingAt = string                              // RFC 3339
	aliveAt = string                                // RFC 3339
	terminatedAt = string                           // RFC 3339
}

`session:${ID}:capabilities` = Hashes {
	requested = string                              // JSON
	actual = string                                 // JSON
}

`session:${ID}:metadata` = Hashes {
	name = string
	build = string
}

`session:${ID}:upstream` = Hashes {
	host = string
	port = number
	driverSessionID = string
}

`session:${ID}:downstream` = Hashes {
	host = string
	userAgent = string
	lastSeen = string                               // RFC 3339
}

`session:${ID}:storage` = string                    // storage ID
```

### Log event codes

During the lifecycle of a session each component generates status codes for tracing purposes.

#### Node

| Level | Code       | Description                            |
|:------|:-----------|:---------------------------------------|
| Info  |            |                                        |
|       | `BOOT`     | node has become active                 |
|       | `DSTART`   | driver in startup                      |
|       | `DALIVE`   | driver has become responsive           |
|       | `LSINIT`   | local session created                  |
|       | `CLOSED`   | session closed by downstream client    |
|       | `HALT`     | node enters shutdown                   |
| Fail  |            |                                        |
|       | `DTIMEOUT` | driver has not become responsive       |
|       | `DFAILURE` | driver process reported an error       |
|       | `STIMEOUT` | session has been inactive too long     |
|       | `TERM`     | node terminates due to fault condition |

#### Orchestrator
| Level | Code        | Description                         |
|:------|:------------|:------------------------------------|
| Info  |             |                                     |
|       | `SCHED`     | node is being scheduled for startup |
| Fail  |             |                                     |
|       | `STARTFAIL` | creation/startup failure            |

#### Manager
| Level | Code           | Description                                         |
|:------|:---------------|:----------------------------------------------------|
| Info  |                |                                                     |
|       | `QUEUED`       | session has been queued at orchestrators            |
|       | `NALLOC`       | node slot has been allocated                        |
|       | `PENDING`      | awaiting node startup                               |
|       | `NALIVE`       | node has become responsive, client served           |
| Warn  |                |                                                     |
|       | `CLEFT`        | client left before scheduling completed             |
| Fail  |                |                                                     |
|       | `INVALIDCAP`   | invalid capabilities requested                      |
|       | `QUNAVAILABLE` | no orchestrator can satisfy the capabilities        |
|       | `QTIMEOUT`     | timed out waiting in queue                          |
|       | `OTIMEOUT`     | timed out waiting for orchestrator to schedule node |
|       | `NTIMEOUT`     | timed out waiting for node to become responsive     |

## Orchestrators
```javascript
`orchestrator:${ID}` = Hashes {
	type = 'local' | 'docker' | 'k8s'
}

`orchestrator:${ID}:heartbeat` = number EX 60
`orchestrator:${ID}:retain` = number EX 604800 				// If this key is not set, the orchestrator metadata can be purged
															// Expires after 7 days and is refreshed by a live orchestrator.

`orchestrator:${ID}:capabilities:platformName` = string
`orchestrator:${ID}:capabilities:browsers` = Set<string>    // explained below

`orchestrator:${ID}:slots.reclaimed` = List<string>         // slot ID
`orchestrator:${ID}:slots.available` = List<string>         // slot ID
`orchestrator:${ID}:slots` = Set<string>                    // slot ID

`orchestrator:${ID}:backlog` = List<string>                 // session ID
`orchestrator:${ID}:pending` = List<string>                 // session ID
```

*Browsers are represented by a string containing the `browserName` and `browserVersion` separated by `::`. For example `chrome::81.0.4044.113` or `firefox::74.0.1`.

[Reliable queue documentation](https://redis.io/commands/rpoplpush#pattern-reliable-queue)

## Metrics

### HTTP (at proxy)
```javascript
`metrics:http:requestsTotal:${method}` = Hashes {
	<http-status-code> = number
}

`metrics:http:net.bytes.total` = Hashes {
	in = number
	out = number
}
```

### Sessions
```javascript
`metrics:sessions:total` = Hashes {
	queued = number
	pending = number
	alive = number
	terminated = number
}

// TODO Figure out how to actually set this, maybe on state change based on the previous state?
`metrics:sessions:duration.seconds.total` = Hashes {
	queued = number
	pending = number
	alive = number
}

`metrics:sessions:startup.histogram:count`
`metrics:sessions:startup.histogram:sum`
`metrics:sessions:startup.histogram:buckets` = Hashes {
	<bucket> = number
}

`metrics:sessions:log:${level}` = Hashes {
	<session-log-code> = number
}
```

### Slots
```javascript
`metrics:slots:reclaimed.total` = Hashes {
	dead = number
	orphaned = number
}
```