# Database

## Keys

All metadata is stored in a key-value in-memory database called [Redis](https://redis.io).  Below is a list of all keys that are currently in use, annotated with their type and format (if applicable).

### Root lists
```javascript
`orchestrators` = Set<string>       // uuids
`sessions.active` = Set<string>     // uuids
`sessions.terminated` = Set<string> // uuids
```

### Storage
```javascript
`storage:${SID}:metadata.pending` = List<FileMetadata>	// see below
```

To optimise storage performance and reduce the need for database synchronization, metadata of newly created and modified files is not written to the storage database by the writing service directly. Instead, the relevant metadata is collected and appended to the `:metadata.pending` list. The corresponding storage service will then update its internal database with this information by continously watching this list and pulling new metadata.

### Sessions
```javascript
// ID = unique, external session identifier
`session:${ID}:heartbeat.node` = string EX 60s      // RFC 3339
`session:${ID}:heartbeat.manager` = string EX 30s   // RFC 3339

`session:${ID}:slot` = string                       // slot ID
`session:${ID}:orchestrator` = List<string>         // orchestrator ID

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

`session:${ID}:storage` = string                    // storage ID

`session:${ID}:telemetry.creation` = Hashes {
	traceID = string								// root span / trace ID
	context = string								// serialized span context
}
```

### Orchestrators
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

### Metrics

#### HTTP (at proxy)
```javascript
`metrics:http:requestsTotal:${method}` = Hashes {
	<http-status-code> = number
}

`metrics:http:net.bytes.total` = Hashes {
	in = number
	out = number
}
```

#### Sessions
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

#### Orchestrator
```javascript
`metrics:slots:reclaimed.total` = Hashes {
	dead = number
	orphaned = number
}
```

#### Storage
```javascript
`metrics:storage:disk.bytes.total` = Hashes {
	<storage-id> = number
}

`metrics:storage:disk.bytes.used` = Hashes {
	<storage-id> = number
}
```

## Garbage collection

Special considerations have been taken to ensure that most keys expire on their own (e.g. active manager/storage/api metadata). Those that do not expire on their own (e.g. sessions) will be purged by a dedicated garbage collector service.