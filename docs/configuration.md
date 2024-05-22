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


## Configuration API

TODO: Document the API.

TODO: Include some documentation on Route and CertBinding configuration.

## Utilities

TODO: Describe any tools to help bootstrap configuration.
