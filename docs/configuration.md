# Configuration

## Static app configuration

The configuration file is in YAML.  The following options are supported:

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
upstream_keepalive_pool_size | Optional | 128 | The number of total connections to keep in the connection pool
proxy.http_bind_addrs | vector of strings | Optional | 0.0.0.0:8080 | The HTTP socket addresses to listen on
proxy.https_bind_addrs | vector of strings | Optional | 0.0.0.0:4433 | The HTTPS socket addresses to listen on
proxy.origin_down_time | number | Optional | 10 | How long (in seconds) to mark an origin down on connection failure
proxy.connection_retry_limit | number | Optional | 1 | The maximum number of times to retry connecting to an origin
cache.max_size | number | Optional | 104857600 (100 MB) | The maximum cache size in bytes
api.bind_addr | string | Optional | 0.0.0.0:5000 | The socket address for the config API to listen on
api.tls | bool | Optional | false | Whether to use TLS for the config API
api.cert | string | Optional | N/A | Path to the certificate file for the config API
api.key | string | Optional | N/A | Path to the key file for the config API
api.mutual_tls | bool | Optional | false | If mutual TLS is enabled, the path to the client certificate file

Example configuration: [conf.yaml](../examples/conf.yaml)

## Configuration API

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

Example route to httpbin.org: [route-httpbin.json](../examples/route-httpbin.json)

### POST `route/delete`

Delete a route.  The request body should contain the route name.

### POST `cert/add`

Add or update certificate binding.  The request body should contain the following in JSON:

Name | Type | Required? | Default value | Description
--|--|--|--|--
host | string | Required | N/A | The incoming SNI to bind the certificate to
cert | string | Required | N/A | Path to the certificate file
key | string | Required | N/A | Path to the key file

### POST `cert/delete`

Delete a certificate binding.  The request body should contain the host/SNI of the bound certificate

## Utilities

TODO: Describe any tools to help bootstrap configuration.
