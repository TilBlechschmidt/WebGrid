var searchIndex = JSON.parse('{\
"webgrid":{"doc":"Core grid component","i":[[0,"libraries","webgrid","Shared modules used by every service",null,null],[0,"helpers","webgrid::libraries","Helper functions that don\'t belong elsewhere",null,null],[0,"constants","webgrid::libraries::helpers","Constant values",null,null],[17,"PORT_STORAGE","webgrid::libraries::helpers::constants","",null,null],[17,"PORT_NODE","","",null,null],[17,"PORT_METRICS","","",null,null],[17,"PORT_MANAGER","","",null,null],[17,"PORT_PROXY","","",null,null],[0,"keys","webgrid::libraries::helpers","Redis database keys",null,null],[0,"orchestrator","webgrid::libraries::helpers::keys","",null,null],[0,"capabilities","webgrid::libraries::helpers::keys::orchestrator","",null,null],[5,"platform_name","webgrid::libraries::helpers::keys::orchestrator::capabilities","",null,[[],["string",3]]],[5,"browsers","","",null,[[],["string",3]]],[0,"slots","webgrid::libraries::helpers::keys::orchestrator","",null,null],[5,"allocated","webgrid::libraries::helpers::keys::orchestrator::slots","",null,[[],["string",3]]],[5,"available","","",null,[[],["string",3]]],[5,"reclaimed","","",null,[[],["string",3]]],[3,"LIST","webgrid::libraries::helpers::keys::orchestrator","",null,null],[5,"metadata","","",null,[[],["string",3]]],[5,"backlog","","",null,[[],["string",3]]],[5,"pending","","",null,[[],["string",3]]],[5,"heartbeat","","",null,[[],["string",3]]],[0,"manager","webgrid::libraries::helpers::keys","",null,null],[3,"LIST","webgrid::libraries::helpers::keys::manager","",null,null],[5,"manager_prefix","","",null,[[],["string",3]]],[5,"metadata","","",null,[[],["string",3]]],[5,"heartbeat","","",null,[[],["string",3]]],[0,"session","webgrid::libraries::helpers::keys","",null,null],[0,"heartbeat","webgrid::libraries::helpers::keys::session","",null,null],[5,"manager","webgrid::libraries::helpers::keys::session::heartbeat","",null,[[],["string",3]]],[5,"node","","",null,[[],["string",3]]],[3,"LIST_ACTIVE","webgrid::libraries::helpers::keys::session","",null,null],[5,"status","","",null,[[],["string",3]]],[5,"capabilities","","",null,[[],["string",3]]],[5,"upstream","","",null,[[],["string",3]]],[5,"downstream","","",null,[[],["string",3]]],[5,"slot","","",null,[[],["string",3]]],[5,"orchestrator","","",null,[[],["string",3]]],[5,"storage","","",null,[[],["string",3]]],[0,"storage","webgrid::libraries::helpers::keys","",null,null],[5,"host","webgrid::libraries::helpers::keys::storage","",null,[[],["string",3]]],[0,"lua","webgrid::libraries::helpers","Lua functions used for Redis interaction",null,null],[5,"terminate_session","webgrid::libraries::helpers::lua","Lua script to clean up a session object in the redis …",null,[[],["string",3]]],[5,"fetch_orchestrator_from_session","","Lua script to extract the orchestrator from a session",null,[[],["string",3]]],[3,"Backoff","webgrid::libraries::helpers","Exponential backoff iterator",null,null],[3,"CapabilityTimeouts","","Timeout values for requests to the browser",null,null],[12,"script","","Determines when to interrupt a script that is being …",0,null],[12,"page_load","","Provides the timeout limit used to interrupt navigation …",0,null],[12,"implicit","","Gives the timeout of when to abort locating an element.",0,null],[4,"CapabilityPageLoadStrategy","","Describes which DOM event is used to determine whether or …",null,null],[13,"None","","Only page download, no parsing or asset loading",1,null],[13,"Eager","","Wait until all HTML content has been parsed, discarding …",1,null],[13,"Normal","","Wait until all assets have been parsed and executed",1,null],[4,"CapabilityUnhandledPromptBehavior","","How popups like alerts or prompts should be handled",null,null],[13,"Dismiss","","All simple dialogs encountered should be dismissed.",2,null],[13,"Accept","","All simple dialogs encountered should be accepted.",2,null],[13,"DismissAndNotify","","All simple dialogs encountered should be dismissed, and …",2,null],[13,"AcceptAndNotify","","All simple dialogs encountered should be accepted, and an …",2,null],[13,"Ignore","","All simple dialogs encountered should be left to the user …",2,null],[3,"CapabilitiesProxy","","HTTP proxy settings",null,null],[12,"proxy_type","","Indicates the type of proxy configuration.",3,null],[12,"proxy_autoconfig_url","","Defines the URL for a proxy auto-config file if …",3,null],[12,"ftp_proxy","","Defines the proxy host for FTP traffic when the …",3,null],[12,"http_proxy","","Defines the proxy host for HTTP traffic when the …",3,null],[12,"no_proxy","","Lists the address for which the proxy should be bypassed …",3,null],[12,"ssl_proxy","","Defines the proxy host for encrypted TLS traffic when the …",3,null],[12,"socks_proxy","","Defines the proxy host for a SOCKS proxy when the …",3,null],[12,"socks_version","","Defines the SOCKS proxy version when the proxy_type is …",3,null],[3,"Capabilities","","Struct containing information about the browser requested …",null,null],[12,"strict_file_interactability","","Indicates if strict interactability checks should be …",4,null],[12,"accept_insecure_certs","","Indicates whether untrusted and self-signed TLS …",4,null],[12,"browser_name","","Identifies the user agent.",4,null],[12,"browser_version","","Identifies the version of the user agent.",4,null],[12,"platform_name","","Identifies the operating system of the endpoint node.",4,null],[12,"page_load_strategy","","Defines the current session’s page load strategy.",4,null],[12,"proxy","","Defines the current session’s proxy configuration.",4,null],[12,"timeouts","","Describes the timeouts imposed on certain session …",4,null],[12,"unhandled_prompt_behavior","","Describes the current session’s user prompt handler.",4,null],[12,"extension_capabilities","","Additional capabilities that are not part of the W3C …",4,null],[3,"CapabilitiesRequest","","List of requested capabilities by a client",null,null],[12,"first_match","","",5,null],[12,"always_match","","",5,null],[5,"wait_for","","Sends HTTP requests to the specified URL until either a …",null,[[["duration",3]]]],[4,"Timeout","","Timeout value accessors in seconds",null,null],[13,"Queue","","How long a session creation request may wait for an …",6,null],[13,"Scheduling","","Maximum duration a session may take to be scheduled by an …",6,null],[13,"NodeStartup","","How long a session may take to start up",6,null],[13,"DriverStartup","","How long the WebDriver executable may take to become …",6,null],[13,"SessionTermination","","Maximum idle duration of a session",6,null],[13,"SlotReclaimInterval","","Interval at which orphaned slots are reclaimed",6,null],[5,"split_into_two","","Splits the input string into two parts at the first …",null,[[],["option",4]]],[5,"parse_browser_string","","Parses a browser string into a name and version",null,[[],["option",4]]],[5,"load_config","","Reads a config file by name from the default config …",null,[[],["string",3]]],[5,"replace_config_variable","","Replaces a variable in the passed config string",null,[[["string",3]],["string",3]]],[0,"lifecycle","webgrid::libraries","Service lifecycle functions",null,null],[0,"logging","webgrid::libraries::lifecycle","Structures for logging to database",null,null],[3,"Logger","webgrid::libraries::lifecycle::logging","Database logging facility",null,null],[11,"new","","Creates a new database logger for the specified component",7,[[["string",3]],["logger",3]]],[11,"log","","Write a log message to the database",7,[[["string",3],["option",4],["logcode",4]]]],[3,"SessionLogger","","Wrapper around logger that stores the session_id",null,null],[11,"new","","Creates a new database logger for the specified component …",8,[[["string",3]],["sessionlogger",3]]],[11,"log","","Write a log message to the database",8,[[["string",3],["option",4],["logcode",4]]]],[4,"LogLevel","","Message criticality",null,null],[13,"INFO","","",9,null],[13,"WARN","","",9,null],[13,"FAIL","","",9,null],[4,"LogCode","","Log event types",null,null],[13,"FAILURE","","",10,null],[13,"BOOT","","node has become active",10,null],[13,"DSTART","","driver in startup",10,null],[13,"DALIVE","","driver has become responsive",10,null],[13,"LSINIT","","local session created",10,null],[13,"CLOSED","","session closed by downstream client",10,null],[13,"HALT","","node enters shutdown",10,null],[13,"DTIMEOUT","","driver has not become responsive",10,null],[13,"DFAILURE","","driver process reported an error",10,null],[13,"STIMEOUT","","session has been inactive too long",10,null],[13,"TERM","","node terminates due to fault condition",10,null],[13,"SCHED","","node is being scheduled for startup",10,null],[13,"STARTFAIL","","creation/startup failure",10,null],[13,"QUEUED","","session has been queued at orchestrators",10,null],[13,"NALLOC","","node slot has been allocated",10,null],[13,"PENDING","","awaiting node startup",10,null],[13,"NALIVE","","node has become responsive, client served",10,null],[13,"CLEFT","","client left before scheduling completed",10,null],[13,"INVALIDCAP","","invalid capabilities requested",10,null],[13,"QUNAVAILABLE","","no orchestrator can satisfy the capabilities",10,null],[13,"QTIMEOUT","","timed out waiting in queue",10,null],[13,"OTIMEOUT","","timed out waiting for orchestrator to schedule node",10,null],[13,"NTIMEOUT","","timed out waiting for node to become responsive",10,null],[11,"level","","Log level for a given event type",10,[[],["loglevel",4]]],[3,"Heart","webgrid::libraries::lifecycle","Lifecycle management struct that can be used to keep the …",null,null],[3,"HeartStone","","Remote controller for the heart",null,null],[4,"BeatValue","","Content of a heartbeat",null,null],[13,"Timestamp","","",11,null],[13,"Constant","","",11,null],[3,"HeartBeat","","Job which keeps heartbeats in the database up-to-date",null,null],[0,"resources","webgrid::libraries","Monitored resources for jobs",null,null],[3,"DefaultResourceManager","webgrid::libraries::resources","Production resource manager",null,null],[3,"RedisResource","","Redis connection that monitors for connection errors",null,null],[6,"SharedRedisResource","","Multiplexed redis resource shared between jobs",null,null],[6,"StandaloneRedisResource","","Individual redis resource created on-demand",null,null],[6,"PubSub","","Boxed PubSubResource shorthand",null,null],[8,"PubSubResource","","Redis PubSub channel resource",null,null],[10,"psubscribe","","",12,[[],[["pin",3],["box",3]]]],[10,"on_message","","",12,[[],[["pin",3],["box",3]]]],[4,"PubSubResourceError","","PubSub listening errors",null,null],[13,"StreamClosed","","",13,null],[8,"ResourceManager","","Manager that provides access to a set of resources",null,null],[16,"Redis","","",14,null],[16,"SharedRedis","","",14,null],[10,"redis","","",14,[[["taskresourcehandle",3]],[["box",3],["pin",3]]]],[10,"shared_redis","","",14,[[["taskresourcehandle",3]],[["box",3],["pin",3]]]],[6,"ResourceManagerResult","","Result shorthand",null,null],[0,"scheduling","webgrid::libraries","Job handling and scheduling structs",null,null],[8,"Job","webgrid::libraries::scheduling","Persistent execution unit",null,null],[16,"Context","","",15,null],[18,"NAME","","Name of the job displayed in log messages",15,null],[18,"SUPPORTS_GRACEFUL_TERMINATION","","Whether or not the job honors the termination signal. …",15,null],[11,"name","","",15,[[]]],[11,"supports_graceful_termination","","",15,[[]]],[10,"execute","","",15,[[["taskmanager",3]],[["box",3],["pin",3]]]],[3,"JobScheduler","","Job and task lifecycle handler",null,null],[3,"StatusServer","","HTTP healthcheck server",null,null],[3,"TaskManager","","Manager for tasks and jobs",null,null],[12,"context","","",16,null],[3,"TaskResourceHandle","","Notification handle for a resource",null,null],[0,"storage","webgrid::libraries","File system accessors",null,null],[4,"StorageError","webgrid::libraries::storage","Errors thrown during filesystem access",null,null],[13,"DatabaseInaccessible","","",17,null],[13,"InternalError","","",17,null],[13,"StripPrefixError","","",17,null],[13,"IOError","","",17,null],[3,"StorageHandler","","Filesystem accessor",null,null],[0,"services","webgrid","Individual micro-services for the grid",null,null],[0,"proxy","webgrid::services","Unified grid entrypoint provider",null,null],[3,"Options","webgrid::services::proxy","Unified grid entrypoint provider",null,null],[5,"run","","",null,[[["sharedoptions",3],["options",3]]]],[0,"manager","webgrid::services","Endpoint for handling session creation",null,null],[4,"RequestError","webgrid::services::manager","",null,null],[13,"RedisError","","",18,null],[13,"QueueTimeout","","",18,null],[13,"SchedulingTimeout","","",18,null],[13,"HealthCheckTimeout","","",18,null],[13,"ParseError","","",18,null],[13,"NoOrchestratorAvailable","","",18,null],[13,"InvalidCapabilities","","",18,null],[13,"ResourceUnavailable","","",18,null],[3,"SessionRequest","","",null,null],[12,"capabilities","","",19,null],[3,"SessionReplyValue","","",null,null],[12,"session_id","","",20,null],[12,"capabilities","","",20,null],[3,"SessionReplyError","","",null,null],[12,"error","","",21,null],[12,"message","","",21,null],[3,"SessionReply","","",null,null],[12,"value","","",22,null],[3,"Options","","Endpoint for handling session creation",null,null],[5,"run","","",null,[[["sharedoptions",3],["options",3]]]],[0,"node","webgrid::services","Session provider and driver manager",null,null],[3,"Options","webgrid::services::node","Session provider",null,null],[5,"run","","",null,[[["sharedoptions",3],["options",3]]]],[0,"metrics","webgrid::services","Prometheus metric provider",null,null],[3,"Options","webgrid::services::metrics","Prometheus metric provider",null,null],[5,"run","","",null,[[["sharedoptions",3],["options",3]]]],[0,"orchestrator","webgrid::services","Provisioner for new session nodes",null,null],[0,"provisioners","webgrid::services::orchestrator","",null,null],[0,"docker","webgrid::services::orchestrator::provisioners","",null,null],[3,"Options","webgrid::services::orchestrator::provisioners::docker","Docker container based session provisioner",null,null],[5,"run","","",null,[[["sharedoptions",3],["options",3],["coreoptions",3]]]],[0,"kubernetes","webgrid::services::orchestrator::provisioners","",null,null],[3,"Options","webgrid::services::orchestrator::provisioners::kubernetes","Kubernetes job based session provisioner",null,null],[5,"run","","",null,[[["sharedoptions",3],["options",3],["coreoptions",3]]]],[3,"Options","webgrid::services::orchestrator","",null,null],[12,"core","","",23,null],[12,"provisioner","","",23,null],[4,"Provisioner","","",null,null],[13,"Docker","","",24,null],[13,"Kubernetes","","",24,null],[0,"storage","webgrid::services","Content delivery service",null,null],[3,"Options","webgrid::services::storage","Content delivery service",null,null],[5,"run","","",null,[[["sharedoptions",3],["options",3]]]],[3,"SharedOptions","webgrid::services","",null,null],[12,"redis","","Redis database server URL",25,null],[12,"status_server","","Enable status reporting server with optional port.",25,null],[12,"log","","Log level, scopable to different modules",25,null],[14,"with_redis_resource","webgrid","Shorthand to request a redis resource from a manager",null,null],[14,"with_shared_redis_resource","","Shorthand to request a shared redis resource from a …",null,null],[14,"schedule","","Schedule jobs on a given scheduler with some context",null,null],[11,"from","webgrid::libraries::helpers","",26,[[]]],[11,"into","","",26,[[]]],[11,"into_iter","","",26,[[]]],[11,"borrow","","",26,[[]]],[11,"borrow_mut","","",26,[[]]],[11,"try_from","","",26,[[],["result",4]]],[11,"try_into","","",26,[[],["result",4]]],[11,"type_id","","",26,[[],["typeid",3]]],[11,"vzip","","",26,[[]]],[11,"zip_option","","",26,[[],["zipoption",3]]],[11,"from","","",0,[[]]],[11,"into","","",0,[[]]],[11,"to_owned","","",0,[[]]],[11,"clone_into","","",0,[[]]],[11,"borrow","","",0,[[]]],[11,"borrow_mut","","",0,[[]]],[11,"try_from","","",0,[[],["result",4]]],[11,"try_into","","",0,[[],["result",4]]],[11,"type_id","","",0,[[],["typeid",3]]],[11,"vzip","","",0,[[]]],[11,"from","","",1,[[]]],[11,"into","","",1,[[]]],[11,"to_owned","","",1,[[]]],[11,"clone_into","","",1,[[]]],[11,"borrow","","",1,[[]]],[11,"borrow_mut","","",1,[[]]],[11,"try_from","","",1,[[],["result",4]]],[11,"try_into","","",1,[[],["result",4]]],[11,"type_id","","",1,[[],["typeid",3]]],[11,"vzip","","",1,[[]]],[11,"from","","",2,[[]]],[11,"into","","",2,[[]]],[11,"to_owned","","",2,[[]]],[11,"clone_into","","",2,[[]]],[11,"borrow","","",2,[[]]],[11,"borrow_mut","","",2,[[]]],[11,"try_from","","",2,[[],["result",4]]],[11,"try_into","","",2,[[],["result",4]]],[11,"type_id","","",2,[[],["typeid",3]]],[11,"vzip","","",2,[[]]],[11,"from","","",3,[[]]],[11,"into","","",3,[[]]],[11,"to_owned","","",3,[[]]],[11,"clone_into","","",3,[[]]],[11,"borrow","","",3,[[]]],[11,"borrow_mut","","",3,[[]]],[11,"try_from","","",3,[[],["result",4]]],[11,"try_into","","",3,[[],["result",4]]],[11,"type_id","","",3,[[],["typeid",3]]],[11,"vzip","","",3,[[]]],[11,"from","","",4,[[]]],[11,"into","","",4,[[]]],[11,"borrow","","",4,[[]]],[11,"borrow_mut","","",4,[[]]],[11,"try_from","","",4,[[],["result",4]]],[11,"try_into","","",4,[[],["result",4]]],[11,"type_id","","",4,[[],["typeid",3]]],[11,"vzip","","",4,[[]]],[11,"from","","",5,[[]]],[11,"into","","",5,[[]]],[11,"borrow","","",5,[[]]],[11,"borrow_mut","","",5,[[]]],[11,"try_from","","",5,[[],["result",4]]],[11,"try_into","","",5,[[],["result",4]]],[11,"type_id","","",5,[[],["typeid",3]]],[11,"vzip","","",5,[[]]],[11,"from","","",6,[[]]],[11,"into","","",6,[[]]],[11,"to_string","","",6,[[],["string",3]]],[11,"borrow","","",6,[[]]],[11,"borrow_mut","","",6,[[]]],[11,"try_from","","",6,[[],["result",4]]],[11,"try_into","","",6,[[],["result",4]]],[11,"type_id","","",6,[[],["typeid",3]]],[11,"vzip","","",6,[[]]],[11,"from","webgrid::libraries::helpers::keys::orchestrator","",27,[[]]],[11,"into","","",27,[[]]],[11,"borrow","","",27,[[]]],[11,"borrow_mut","","",27,[[]]],[11,"try_from","","",27,[[],["result",4]]],[11,"try_into","","",27,[[],["result",4]]],[11,"type_id","","",27,[[],["typeid",3]]],[11,"vzip","","",27,[[]]],[11,"from","webgrid::libraries::helpers::keys::manager","",28,[[]]],[11,"into","","",28,[[]]],[11,"borrow","","",28,[[]]],[11,"borrow_mut","","",28,[[]]],[11,"try_from","","",28,[[],["result",4]]],[11,"try_into","","",28,[[],["result",4]]],[11,"type_id","","",28,[[],["typeid",3]]],[11,"vzip","","",28,[[]]],[11,"from","webgrid::libraries::helpers::keys::session","",29,[[]]],[11,"into","","",29,[[]]],[11,"borrow","","",29,[[]]],[11,"borrow_mut","","",29,[[]]],[11,"try_from","","",29,[[],["result",4]]],[11,"try_into","","",29,[[],["result",4]]],[11,"type_id","","",29,[[],["typeid",3]]],[11,"vzip","","",29,[[]]],[11,"from","webgrid::libraries::lifecycle","",30,[[]]],[11,"into","","",30,[[]]],[11,"borrow","","",30,[[]]],[11,"borrow_mut","","",30,[[]]],[11,"try_from","","",30,[[],["result",4]]],[11,"try_into","","",30,[[],["result",4]]],[11,"type_id","","",30,[[],["typeid",3]]],[11,"vzip","","",30,[[]]],[11,"from","","",31,[[]]],[11,"into","","",31,[[]]],[11,"to_owned","","",31,[[]]],[11,"clone_into","","",31,[[]]],[11,"borrow","","",31,[[]]],[11,"borrow_mut","","",31,[[]]],[11,"try_from","","",31,[[],["result",4]]],[11,"try_into","","",31,[[],["result",4]]],[11,"type_id","","",31,[[],["typeid",3]]],[11,"vzip","","",31,[[]]],[11,"from","","",11,[[]]],[11,"into","","",11,[[]]],[11,"borrow","","",11,[[]]],[11,"borrow_mut","","",11,[[]]],[11,"try_from","","",11,[[],["result",4]]],[11,"try_into","","",11,[[],["result",4]]],[11,"type_id","","",11,[[],["typeid",3]]],[11,"vzip","","",11,[[]]],[11,"from","","",32,[[]]],[11,"into","","",32,[[]]],[11,"to_owned","","",32,[[]]],[11,"clone_into","","",32,[[]]],[11,"borrow","","",32,[[]]],[11,"borrow_mut","","",32,[[]]],[11,"try_from","","",32,[[],["result",4]]],[11,"try_into","","",32,[[],["result",4]]],[11,"type_id","","",32,[[],["typeid",3]]],[11,"vzip","","",32,[[]]],[11,"from","webgrid::libraries::lifecycle::logging","",7,[[]]],[11,"into","","",7,[[]]],[11,"borrow","","",7,[[]]],[11,"borrow_mut","","",7,[[]]],[11,"try_from","","",7,[[],["result",4]]],[11,"try_into","","",7,[[],["result",4]]],[11,"type_id","","",7,[[],["typeid",3]]],[11,"vzip","","",7,[[]]],[11,"from","","",8,[[]]],[11,"into","","",8,[[]]],[11,"borrow","","",8,[[]]],[11,"borrow_mut","","",8,[[]]],[11,"try_from","","",8,[[],["result",4]]],[11,"try_into","","",8,[[],["result",4]]],[11,"type_id","","",8,[[],["typeid",3]]],[11,"vzip","","",8,[[]]],[11,"from","","",9,[[]]],[11,"into","","",9,[[]]],[11,"to_string","","",9,[[],["string",3]]],[11,"borrow","","",9,[[]]],[11,"borrow_mut","","",9,[[]]],[11,"try_from","","",9,[[],["result",4]]],[11,"try_into","","",9,[[],["result",4]]],[11,"type_id","","",9,[[],["typeid",3]]],[11,"vzip","","",9,[[]]],[11,"from","","",10,[[]]],[11,"into","","",10,[[]]],[11,"to_string","","",10,[[],["string",3]]],[11,"borrow","","",10,[[]]],[11,"borrow_mut","","",10,[[]]],[11,"try_from","","",10,[[],["result",4]]],[11,"try_into","","",10,[[],["result",4]]],[11,"type_id","","",10,[[],["typeid",3]]],[11,"vzip","","",10,[[]]],[11,"from","webgrid::libraries::resources","",33,[[]]],[11,"into","","",33,[[]]],[11,"to_owned","","",33,[[]]],[11,"clone_into","","",33,[[]]],[11,"borrow","","",33,[[]]],[11,"borrow_mut","","",33,[[]]],[11,"try_from","","",33,[[],["result",4]]],[11,"try_into","","",33,[[],["result",4]]],[11,"type_id","","",33,[[],["typeid",3]]],[11,"vzip","","",33,[[]]],[11,"from","","",34,[[]]],[11,"into","","",34,[[]]],[11,"borrow","","",34,[[]]],[11,"borrow_mut","","",34,[[]]],[11,"try_from","","",34,[[],["result",4]]],[11,"try_into","","",34,[[],["result",4]]],[11,"type_id","","",34,[[],["typeid",3]]],[11,"vzip","","",34,[[]]],[11,"from","","",13,[[]]],[11,"into","","",13,[[]]],[11,"borrow","","",13,[[]]],[11,"borrow_mut","","",13,[[]]],[11,"try_from","","",13,[[],["result",4]]],[11,"try_into","","",13,[[],["result",4]]],[11,"type_id","","",13,[[],["typeid",3]]],[11,"vzip","","",13,[[]]],[11,"from","webgrid::libraries::scheduling","",35,[[]]],[11,"into","","",35,[[]]],[11,"borrow","","",35,[[]]],[11,"borrow_mut","","",35,[[]]],[11,"try_from","","",35,[[],["result",4]]],[11,"try_into","","",35,[[],["result",4]]],[11,"type_id","","",35,[[],["typeid",3]]],[11,"vzip","","",35,[[]]],[11,"from","","",36,[[]]],[11,"into","","",36,[[]]],[11,"to_owned","","",36,[[]]],[11,"clone_into","","",36,[[]]],[11,"borrow","","",36,[[]]],[11,"borrow_mut","","",36,[[]]],[11,"try_from","","",36,[[],["result",4]]],[11,"try_into","","",36,[[],["result",4]]],[11,"type_id","","",36,[[],["typeid",3]]],[11,"vzip","","",36,[[]]],[11,"from","","",16,[[]]],[11,"into","","",16,[[]]],[11,"to_owned","","",16,[[]]],[11,"clone_into","","",16,[[]]],[11,"borrow","","",16,[[]]],[11,"borrow_mut","","",16,[[]]],[11,"try_from","","",16,[[],["result",4]]],[11,"try_into","","",16,[[],["result",4]]],[11,"type_id","","",16,[[],["typeid",3]]],[11,"vzip","","",16,[[]]],[11,"from","","",37,[[]]],[11,"into","","",37,[[]]],[11,"to_owned","","",37,[[]]],[11,"clone_into","","",37,[[]]],[11,"borrow","","",37,[[]]],[11,"borrow_mut","","",37,[[]]],[11,"try_from","","",37,[[],["result",4]]],[11,"try_into","","",37,[[],["result",4]]],[11,"type_id","","",37,[[],["typeid",3]]],[11,"equivalent","","",37,[[]]],[11,"get_hash","","",37,[[]]],[11,"get_hash","","",37,[[]]],[11,"vzip","","",37,[[]]],[11,"from","webgrid::libraries::storage","",38,[[]]],[11,"into","","",38,[[]]],[11,"to_owned","","",38,[[]]],[11,"clone_into","","",38,[[]]],[11,"borrow","","",38,[[]]],[11,"borrow_mut","","",38,[[]]],[11,"try_from","","",38,[[],["result",4]]],[11,"try_into","","",38,[[],["result",4]]],[11,"type_id","","",38,[[],["typeid",3]]],[11,"vzip","","",38,[[]]],[11,"from","","",17,[[]]],[11,"into","","",17,[[]]],[11,"to_string","","",17,[[],["string",3]]],[11,"borrow","","",17,[[]]],[11,"borrow_mut","","",17,[[]]],[11,"try_from","","",17,[[],["result",4]]],[11,"try_into","","",17,[[],["result",4]]],[11,"type_id","","",17,[[],["typeid",3]]],[11,"vzip","","",17,[[]]],[11,"from","webgrid::services::proxy","",39,[[]]],[11,"into","","",39,[[]]],[11,"borrow","","",39,[[]]],[11,"borrow_mut","","",39,[[]]],[11,"try_from","","",39,[[],["result",4]]],[11,"try_into","","",39,[[],["result",4]]],[11,"type_id","","",39,[[],["typeid",3]]],[11,"vzip","","",39,[[]]],[11,"from","webgrid::services::manager","",18,[[]]],[11,"into","","",18,[[]]],[11,"to_string","","",18,[[],["string",3]]],[11,"borrow","","",18,[[]]],[11,"borrow_mut","","",18,[[]]],[11,"try_from","","",18,[[],["result",4]]],[11,"try_into","","",18,[[],["result",4]]],[11,"type_id","","",18,[[],["typeid",3]]],[11,"vzip","","",18,[[]]],[11,"from","","",19,[[]]],[11,"into","","",19,[[]]],[11,"borrow","","",19,[[]]],[11,"borrow_mut","","",19,[[]]],[11,"try_from","","",19,[[],["result",4]]],[11,"try_into","","",19,[[],["result",4]]],[11,"type_id","","",19,[[],["typeid",3]]],[11,"vzip","","",19,[[]]],[11,"from","","",20,[[]]],[11,"into","","",20,[[]]],[11,"borrow","","",20,[[]]],[11,"borrow_mut","","",20,[[]]],[11,"try_from","","",20,[[],["result",4]]],[11,"try_into","","",20,[[],["result",4]]],[11,"type_id","","",20,[[],["typeid",3]]],[11,"vzip","","",20,[[]]],[11,"from","","",21,[[]]],[11,"into","","",21,[[]]],[11,"borrow","","",21,[[]]],[11,"borrow_mut","","",21,[[]]],[11,"try_from","","",21,[[],["result",4]]],[11,"try_into","","",21,[[],["result",4]]],[11,"type_id","","",21,[[],["typeid",3]]],[11,"vzip","","",21,[[]]],[11,"from","","",22,[[]]],[11,"into","","",22,[[]]],[11,"borrow","","",22,[[]]],[11,"borrow_mut","","",22,[[]]],[11,"try_from","","",22,[[],["result",4]]],[11,"try_into","","",22,[[],["result",4]]],[11,"type_id","","",22,[[],["typeid",3]]],[11,"vzip","","",22,[[]]],[11,"from","","",40,[[]]],[11,"into","","",40,[[]]],[11,"borrow","","",40,[[]]],[11,"borrow_mut","","",40,[[]]],[11,"try_from","","",40,[[],["result",4]]],[11,"try_into","","",40,[[],["result",4]]],[11,"type_id","","",40,[[],["typeid",3]]],[11,"vzip","","",40,[[]]],[11,"from","webgrid::services::node","",41,[[]]],[11,"into","","",41,[[]]],[11,"to_owned","","",41,[[]]],[11,"clone_into","","",41,[[]]],[11,"borrow","","",41,[[]]],[11,"borrow_mut","","",41,[[]]],[11,"try_from","","",41,[[],["result",4]]],[11,"try_into","","",41,[[],["result",4]]],[11,"type_id","","",41,[[],["typeid",3]]],[11,"vzip","","",41,[[]]],[11,"from","webgrid::services::metrics","",42,[[]]],[11,"into","","",42,[[]]],[11,"borrow","","",42,[[]]],[11,"borrow_mut","","",42,[[]]],[11,"try_from","","",42,[[],["result",4]]],[11,"try_into","","",42,[[],["result",4]]],[11,"type_id","","",42,[[],["typeid",3]]],[11,"vzip","","",42,[[]]],[11,"from","webgrid::services::orchestrator::provisioners::docker","",43,[[]]],[11,"into","","",43,[[]]],[11,"borrow","","",43,[[]]],[11,"borrow_mut","","",43,[[]]],[11,"try_from","","",43,[[],["result",4]]],[11,"try_into","","",43,[[],["result",4]]],[11,"type_id","","",43,[[],["typeid",3]]],[11,"vzip","","",43,[[]]],[11,"from","webgrid::services::orchestrator::provisioners::kubernetes","",44,[[]]],[11,"into","","",44,[[]]],[11,"borrow","","",44,[[]]],[11,"borrow_mut","","",44,[[]]],[11,"try_from","","",44,[[],["result",4]]],[11,"try_into","","",44,[[],["result",4]]],[11,"type_id","","",44,[[],["typeid",3]]],[11,"vzip","","",44,[[]]],[11,"from","webgrid::services::orchestrator","",23,[[]]],[11,"into","","",23,[[]]],[11,"borrow","","",23,[[]]],[11,"borrow_mut","","",23,[[]]],[11,"try_from","","",23,[[],["result",4]]],[11,"try_into","","",23,[[],["result",4]]],[11,"type_id","","",23,[[],["typeid",3]]],[11,"vzip","","",23,[[]]],[11,"from","","",24,[[]]],[11,"into","","",24,[[]]],[11,"borrow","","",24,[[]]],[11,"borrow_mut","","",24,[[]]],[11,"try_from","","",24,[[],["result",4]]],[11,"try_into","","",24,[[],["result",4]]],[11,"type_id","","",24,[[],["typeid",3]]],[11,"vzip","","",24,[[]]],[11,"from","webgrid::services::storage","",45,[[]]],[11,"into","","",45,[[]]],[11,"borrow","","",45,[[]]],[11,"borrow_mut","","",45,[[]]],[11,"try_from","","",45,[[],["result",4]]],[11,"try_into","","",45,[[],["result",4]]],[11,"type_id","","",45,[[],["typeid",3]]],[11,"vzip","","",45,[[]]],[11,"from","webgrid::services","",25,[[]]],[11,"into","","",25,[[]]],[11,"borrow","","",25,[[]]],[11,"borrow_mut","","",25,[[]]],[11,"try_from","","",25,[[],["result",4]]],[11,"try_into","","",25,[[],["result",4]]],[11,"type_id","","",25,[[],["typeid",3]]],[11,"vzip","","",25,[[]]],[11,"redis","webgrid::libraries::resources","",33,[[["taskresourcehandle",3]],[["box",3],["pin",3]]]],[11,"shared_redis","","",33,[[["taskresourcehandle",3]],[["box",3],["pin",3]]]],[11,"execute","webgrid::libraries::lifecycle","",32,[[["taskmanager",3]],[["box",3],["pin",3]]]],[11,"execute","webgrid::libraries::scheduling","",36,[[["taskmanager",3]],[["box",3],["pin",3]]]],[11,"from","webgrid::libraries::storage","",17,[[["error",4]]]],[11,"from","","",17,[[["stripprefixerror",3]]]],[11,"from","","",17,[[["ioerror",3]]]],[11,"next","webgrid::libraries::helpers","",26,[[],["option",4]]],[11,"clone","","",0,[[],["capabilitytimeouts",3]]],[11,"clone","","",1,[[],["capabilitypageloadstrategy",4]]],[11,"clone","","",2,[[],["capabilityunhandledpromptbehavior",4]]],[11,"clone","","",3,[[],["capabilitiesproxy",3]]],[11,"clone","webgrid::libraries::lifecycle","",31,[[],["heartstone",3]]],[11,"clone","","",32,[[],["heartbeat",3]]],[11,"clone","webgrid::libraries::resources","",33,[[],["defaultresourcemanager",3]]],[11,"clone","webgrid::libraries::scheduling","",36,[[],["statusserver",3]]],[11,"clone","","",16,[[],["taskmanager",3]]],[11,"clone","","",37,[[],["taskresourcehandle",3]]],[11,"clone","webgrid::libraries::storage","",38,[[],["storagehandler",3]]],[11,"clone","webgrid::services::node","",41,[[],["options",3]]],[11,"default","webgrid::libraries::helpers","",26,[[]]],[11,"default","webgrid::libraries::lifecycle","",32,[[]]],[11,"default","webgrid::libraries::scheduling","",35,[[],["jobscheduler",3]]],[11,"eq","","",37,[[]]],[11,"deref","webgrid::libraries::helpers::keys::orchestrator","",27,[[],["string",3]]],[11,"deref","webgrid::libraries::helpers::keys::manager","",28,[[],["string",3]]],[11,"deref","webgrid::libraries::helpers::keys::session","",29,[[],["string",3]]],[11,"fmt","webgrid::libraries::helpers","",0,[[["formatter",3]],["result",6]]],[11,"fmt","","",1,[[["formatter",3]],["result",6]]],[11,"fmt","","",2,[[["formatter",3]],["result",6]]],[11,"fmt","","",3,[[["formatter",3]],["result",6]]],[11,"fmt","","",4,[[["formatter",3]],["result",6]]],[11,"fmt","","",5,[[["formatter",3]],["result",6]]],[11,"fmt","","",6,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::libraries::lifecycle::logging","",9,[[["formatter",3]],["result",6]]],[11,"fmt","","",10,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::libraries::storage","",17,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services::proxy","",39,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services::manager","",18,[[["formatter",3]],["result",6]]],[11,"fmt","","",19,[[["formatter",3]],["result",6]]],[11,"fmt","","",20,[[["formatter",3]],["result",6]]],[11,"fmt","","",40,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services::node","",41,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services::metrics","",42,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services::orchestrator::provisioners::docker","",43,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services::orchestrator::provisioners::kubernetes","",44,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services::orchestrator","",23,[[["formatter",3]],["result",6]]],[11,"fmt","","",24,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services::storage","",45,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services","",25,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::libraries::helpers","",6,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::libraries::lifecycle::logging","",9,[[["formatter",3]],["result",6]]],[11,"fmt","","",10,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::libraries::storage","",17,[[["formatter",3]],["result",6]]],[11,"fmt","webgrid::services::manager","",18,[[["formatter",3]],["result",6]]],[11,"hash","webgrid::libraries::scheduling","",37,[[]]],[11,"source","webgrid::libraries::storage","",17,[[],[["error",8],["option",4]]]],[11,"serialize","webgrid::services::manager","",20,[[],["result",4]]],[11,"serialize","","",21,[[],["result",4]]],[11,"serialize","","",22,[[],["result",4]]],[11,"deserialize","webgrid::libraries::helpers","",0,[[],["result",4]]],[11,"deserialize","","",1,[[],["result",4]]],[11,"deserialize","","",2,[[],["result",4]]],[11,"deserialize","","",3,[[],["result",4]]],[11,"deserialize","","",4,[[],["result",4]]],[11,"deserialize","","",5,[[],["result",4]]],[11,"deserialize","webgrid::services::manager","",19,[[],["result",4]]],[11,"initialize","webgrid::libraries::helpers::keys::orchestrator","",27,[[]]],[11,"initialize","webgrid::libraries::helpers::keys::manager","",28,[[]]],[11,"initialize","webgrid::libraries::helpers::keys::session","",29,[[]]],[11,"req_packed_command","webgrid::libraries::resources","",34,[[["cmd",3]],[["value",4],["redisfuture",6]]]],[11,"req_packed_commands","","",34,[[["pipeline",3]],[["vec",3],["redisfuture",6]]]],[11,"get_db","","",34,[[]]],[11,"clap","webgrid::services::proxy","",39,[[],["app",3]]],[11,"from_clap","","",39,[[["argmatches",3]]]],[11,"clap","webgrid::services::manager","",40,[[],["app",3]]],[11,"from_clap","","",40,[[["argmatches",3]]]],[11,"clap","webgrid::services::node","",41,[[],["app",3]]],[11,"from_clap","","",41,[[["argmatches",3]]]],[11,"clap","webgrid::services::metrics","",42,[[],["app",3]]],[11,"from_clap","","",42,[[["argmatches",3]]]],[11,"clap","webgrid::services::orchestrator::provisioners::docker","",43,[[],["app",3]]],[11,"from_clap","","",43,[[["argmatches",3]]]],[11,"clap","webgrid::services::orchestrator::provisioners::kubernetes","",44,[[],["app",3]]],[11,"from_clap","","",44,[[["argmatches",3]]]],[11,"clap","webgrid::services::orchestrator","",23,[[],["app",3]]],[11,"from_clap","","",23,[[["argmatches",3]]]],[11,"clap","","",24,[[],["app",3]]],[11,"from_clap","","",24,[[["argmatches",3]]]],[11,"clap","webgrid::services::storage","",45,[[],["app",3]]],[11,"from_clap","","",45,[[["argmatches",3]]]],[11,"clap","webgrid::services","",25,[[],["app",3]]],[11,"from_clap","","",25,[[["argmatches",3]]]],[11,"augment_clap","webgrid::services::proxy","",39,[[["app",3]],["app",3]]],[11,"is_subcommand","","",39,[[]]],[11,"augment_clap","webgrid::services::manager","",40,[[["app",3]],["app",3]]],[11,"is_subcommand","","",40,[[]]],[11,"augment_clap","webgrid::services::node","",41,[[["app",3]],["app",3]]],[11,"is_subcommand","","",41,[[]]],[11,"augment_clap","webgrid::services::metrics","",42,[[["app",3]],["app",3]]],[11,"is_subcommand","","",42,[[]]],[11,"augment_clap","webgrid::services::orchestrator::provisioners::docker","",43,[[["app",3]],["app",3]]],[11,"is_subcommand","","",43,[[]]],[11,"augment_clap","webgrid::services::orchestrator::provisioners::kubernetes","",44,[[["app",3]],["app",3]]],[11,"is_subcommand","","",44,[[]]],[11,"augment_clap","webgrid::services::orchestrator","",23,[[["app",3]],["app",3]]],[11,"is_subcommand","","",23,[[]]],[11,"augment_clap","","",24,[[["app",3]],["app",3]]],[11,"from_subcommand","","",24,[[],["option",4]]],[11,"is_subcommand","","",24,[[]]],[11,"augment_clap","webgrid::services::storage","",45,[[["app",3]],["app",3]]],[11,"is_subcommand","","",45,[[]]],[11,"augment_clap","webgrid::services","",25,[[["app",3]],["app",3]]],[11,"is_subcommand","","",25,[[]]],[11,"into_sets","webgrid::libraries::helpers","Converts the request into a set of possible combinations",5,[[],[["capabilities",3],["vec",3]]]],[11,"get","","Retrieve either a value set in the database or …",6,[[]]],[11,"new","webgrid::libraries::lifecycle","Creates a new heart and linked stone with no lifetime …",30,[[]]],[11,"with_lifetime","","Creates a new heart and linked stone with a lifetime",30,[[["duration",3]]]],[11,"death","","Future that waits until the heart dies for the returned …",30,[[]]],[11,"kill","","Kill the associated heart",31,[[["string",3]]]],[11,"reset_lifetime","","Reset the lifetime of the associated heart",31,[[]]],[11,"new","","Creates a new handler with timestamp based heartbeats",32,[[]]],[11,"with_value","","Creates a new handler with a custom value type",32,[[["beatvalue",4]]]],[11,"add_beat","","Add a new beat with a specified interval and expiration …",32,[[]]],[11,"stop_beat","","Remove a heartbeat",32,[[]]],[11,"new","webgrid::libraries::resources","Creates a new resource manager that connects to the redis …",33,[[["string",3]]]],[11,"shared","","Retrieves a shared redis instance or instantiates it if …",34,[[["taskresourcehandle",3]]]],[11,"new","","Creates a new standalone redis connection",34,[[["taskresourcehandle",3]]]],[11,"set_logging","","Enables request logging",34,[[]]],[11,"select","","Set the redis database index",34,[[]]],[18,"NAME","webgrid::libraries::scheduling","Name of the job displayed in log messages",15,null],[18,"SUPPORTS_GRACEFUL_TERMINATION","","Whether or not the job honors the termination signal. …",15,null],[11,"name","","",15,[[]]],[11,"supports_graceful_termination","","",15,[[]]],[11,"spawn_task","","Run a new task with the given context on the default …",35,[[],[["joinhandle",3],["result",4]]]],[11,"spawn_job","","Manage a new job",35,[[["send",8],["job",8]]]],[11,"terminate_jobs","","Gracefully terminates all managed jobs that support it",35,[[]]],[11,"new","","Creates a new server for the given scheduler and port …",36,[[["jobscheduler",3],["option",4],["option",4]]]],[11,"new","","Create a new task manager for the given task and context",16,[[]]],[11,"create_resource_handle","","Create a new resource handle to notify about resource …",16,[[],["taskresourcehandle",3]]],[11,"termination_signal","","Future that completes when the job should gracefully …",16,[[]]],[11,"termination_signal_triggered","","Check if the job should enter graceful shutdown",16,[[]]],[11,"ready","","Function to indicate to the scheduler that this job is …",16,[[]]],[11,"resource_died","","Notifies the task manager that a resource dependency has …",37,[[]]],[11,"storage_id","webgrid::libraries::storage","Fetches the storage ID of the given directory. If the …",38,[[["pathbuf",3]]]],[11,"new","","Creates new instance that watches a given directory and …",38,[[["pathbuf",3]]]],[11,"scan_fs","","Explicitly scan the full filesystem and sync the database",38,[[]]],[11,"add_file","","Import or update a file to the database",38,[[["asref",8],["path",3]]]],[11,"remove_file","","Remove a file from the database",38,[[["asref",8],["path",3]]]],[11,"maybe_cleanup","","Runs a cleanup if the used bytes exceed the <code>size_threshold</code>",38,[[]]]],"p":[[3,"CapabilityTimeouts"],[4,"CapabilityPageLoadStrategy"],[4,"CapabilityUnhandledPromptBehavior"],[3,"CapabilitiesProxy"],[3,"Capabilities"],[3,"CapabilitiesRequest"],[4,"Timeout"],[3,"Logger"],[3,"SessionLogger"],[4,"LogLevel"],[4,"LogCode"],[4,"BeatValue"],[8,"PubSubResource"],[4,"PubSubResourceError"],[8,"ResourceManager"],[8,"Job"],[3,"TaskManager"],[4,"StorageError"],[4,"RequestError"],[3,"SessionRequest"],[3,"SessionReplyValue"],[3,"SessionReplyError"],[3,"SessionReply"],[3,"Options"],[4,"Provisioner"],[3,"SharedOptions"],[3,"Backoff"],[3,"LIST"],[3,"LIST"],[3,"LIST_ACTIVE"],[3,"Heart"],[3,"HeartStone"],[3,"HeartBeat"],[3,"DefaultResourceManager"],[3,"RedisResource"],[3,"JobScheduler"],[3,"StatusServer"],[3,"TaskResourceHandle"],[3,"StorageHandler"],[3,"Options"],[3,"Options"],[3,"Options"],[3,"Options"],[3,"Options"],[3,"Options"],[3,"Options"]]}\
}');
addSearchOptions(searchIndex);initSearch(searchIndex);