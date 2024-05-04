# granite

An HTTP proxy built on Pingora

TODO: Add a logo.

## Features

- Configuration API for dynamically managing routes and certificates.
- Mutual TLS on configuration API.
- HTTP 1.1 and HTTP/2 on downstream and upstream.
- In-memory caching.
- Weighted random load balancing among origins.
- Unreachable origins are temporarily marked down and avoided.
- Origin connection retries.
- Custom SNI and Host header.

## Usage

```
USAGE:
    granite [FLAGS] [OPTIONS]

FLAGS:
    -d, --daemon
            Whether should run this server in the background

    -h, --help
            Prints help information

    -t, --test
            Test the configuration and exit

    -u, --upgrade
            Whether this server should try to upgrade from a running old server

    -V, --version
            Prints version information

OPTIONS:
    -c, --conf <conf>
            The path to the configuration file.
```

Static app configuration (in the configuration file given on the command-line) as well as dynamic
configuration (for routes and certificates) are documented here: [configuration](docs/configuration.md).

## Building

```bash
cargo build
```

## Internals

See the [architecture](docs/architecture.md) documentation.
