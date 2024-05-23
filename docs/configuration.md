# Configuration

Configuration is divided into two parts: static and dynamic.  The static configuration is read from
a file when the server starts and is used to configure the server itself.  The dynamic configuration
is managed through an API and is used to configure routes and certificates.

## Static app configuration

The configuration file (YAML) is given with the command-line option `--conf` (or `-c`).  The config file is divided into
sections, each with its own set of options.

### Top-level options

These include all the Pingora options, most of which are listed here:

Name | Type | Required? | Default value | Description
--|--|--|--|--
version | number | Optional | 1 | The version of the conf.  Only `1` is currently supported
pid_file | string | Optional | N/A | The path to the PID file
daemon | bool | Optional | false | Whether to run the server in the background
error_log | string | Optional | N/A | The path to error log output file. STDERR is used if not set
upgrade_sock | string | Optional | N/A | The path to the upgrade socket
threads | number | Optional | 1 | Number of threads per service
user | string | Optional | N/A | The user the server should be run under after daemonization
group | string | Optional | N/A | The group the server should be run under after daemonization
ca_file | string | Optional | N/A | The path to the root CA file
work_stealing | bool | Optional | true | Enable work stealing runtime
upstream_keepalive_pool_size | number | Optional | 128 | The number of total connections to keep in the connection pool

See the [Pingora documentation](https://docs.rs/pingora-core/latest/pingora_core/server/configuration/struct.ServerConf.html)
for the full list of options.

### Proxy options

These options appear in the `proxy` section of the configuration file.

Name | Type | Required? | Default value | Description
--|--|--|--|--
http_bind_addrs | vector of strings | Optional | 0.0.0.0:8080 | The HTTP socket addresses to listen on
https_bind_addrs | vector of strings | Optional | 0.0.0.0:4433 | The HTTPS socket addresses to listen on
origin_down_time | number | Optional | 10 | How long (in seconds) to mark an origin down on connection failure
connection_retry_limit | number | Optional | 1 | The maximum number of times to retry connecting to an origin

### Cache options

These options appear in the `cache` section of the configuration file.

Name | Type | Required? | Default value | Description
--|--|--|--|--
cache.max_size | number | Optional | 104857600 (100 MB) | The maximum cache size in bytes

### Config API options

These options appear in the `api` section of the configuration file.

Name | Type | Required? | Default value | Description
--|--|--|--|--
api.bind_addr | string | Optional | 0.0.0.0:5000 | The socket address for the config API to listen on
api.tls | bool | Optional | false | Whether to use TLS for the config API
api.cert | string | Optional | N/A | Path to the certificate file for the config API
api.key | string | Optional | N/A | Path to the key file for the config API
api.mutual_tls | bool | Optional | false | If mutual TLS is enabled, the path to the client certificate file

Example configuration: [conf.yaml](../examples/conf.yaml)

## Configuration API

The configuration API is a RESTful API that allows you to add, update, and delete routes and
certificate bindings.

### POST `/route/add`

Add or update a route.  The request body should contain the following in JSON:

Name | Type | Required? | Default value | Description
--|--|--|--|--
name | string | Required | N/A | A name for the route
customer | string | Required | N/A | The customer who owns the route
incoming_schemes | vector of strings | Required | N/A | Accepted schemes: "Http" and/or "Https"
hosts | vector of strings | Required | N/A | A list of hostnames to match the route on
paths | vector of strings | Required | N/A | A list of URI paths prefixes to match the route on
cache | bool | Optional | false | Whether to enable caching for requests matching the route
outgoing_schcme | string | Optional | MatchIncoming | The scheme to use when connecting to the origin ("Http, Https, or MatchIncoming)
origin_group.origins | vector of origins | Required | N/A | See the table below

Origin definition:

Name | Type | Required? | Default value | Description
--|--|--|--|--
host | string | Required | N/A | The hostname or IP address of the origin
http_port | number | Optional | 80 | The HTTP port number of the origin
https_port | number | Optional | 443 | The HTTPS port number of the origin
host_header_override | string | Optional | N/A | The Host header to use when communicating with the origin
sni | string | Optional | N/A | The SNI to use when communicating with the origin
weight | number | Optional | 10 | The relative weight of the origin in the origin group

Example route: [route-forward.json](../examples/route-forward.json)

### POST `route/delete`

Delete a route.  The request body should contain the route name.

### POST `cert/add`

Add or update certificate binding.  The request body should contain the following in JSON:

Name | Type | Required? | Default value | Description
--|--|--|--|--
host | string | Required | N/A | The incoming SNI to bind the certificate to
cert | string | Required | N/A | Path to the certificate file
key | string | Required | N/A | Path to the key file

The tool [`create_cert_binding_json.py`](../examples/create_cert_binding_json.py) can be used to
generate the JSON binding object from a certificate and key file.

### POST `cert/delete`

Delete a certificate binding.  The request body should contain the host/SNI of the bound certificate
