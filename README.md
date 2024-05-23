# granite

An HTTP proxy built on [Pingora](https://github.com/cloudflare/pingora).


<img src="https://github.com/mdevuyst/granite/blob/c2819c9d7c7506f2bf96b194e220a1b72d6879e0/granite.png" alt="logo" width="512"/>

![logo](https://github.com/mdevuyst/granite/blob/c2819c9d7c7506f2bf96b194e220a1b72d6879e0/granite.png)

## Features

- Configuration API for dynamically managing routes and certificates.
- Mutual TLS on configuration API.
- HTTP 1.1 and HTTP/2 on downstream and upstream.
- In-memory caching.
- Weighted random load balancing among origins.
- Unreachable origins are temporarily marked down and avoided.
- Origin connection retries.
- Custom SNI and Host header.

## Quickstart

```bash
# Start the server
RUST_LOG=info cargo run -- --daemon

# Add a simple forwarding route
curl -v -d @examples/route-forward.json http://127.0.0.1:5000/route/add

# Send a request through the proxy
curl -v --connect-to ::127.0.0.1:8080 http://forward/get

# Stop the server
pkill -INT granite
```

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

## Examples

### Caching

```bash
# Start the server
RUST_LOG=info cargo run -- --daemon

# Add a simple caching route
curl -v -d @examples/route-cache.json http://127.0.0.1:5000/route/add

# Send a request through the proxy.
# Notice the `x-cache-status: miss` header in the response.
curl -v --connect-to ::127.0.0.1:8080 http://cache/get

# Send the same request again and expect a cache hit.
# Notice the `x-cache-status: hit` header in the response.
curl -v --connect-to ::127.0.0.1:8080 http://cache/get

# Stop the server
pkill -INT granite
```

### SSL termination

```bash
# Start the server
RUST_LOG=info cargo run -- --daemon

# Add a certificate binding for host `ssl`.
# First, for demonstration purposes, create a self-signed certificate and key.
# Then, wrap the certificate, key, and hostname in a JSON object representing the binding.
# Finally, send the JSON object to the configuration API.
openssl req -x509 -new -nodes -subj "/CN=ssl" -out ssl.crt -keyout ssl.key
python3 examples/create_cert_binding_json.py --host ssl --cert ssl.crt --key ssl.key --output ssl.json
curl -v -d @ssl.json http://127.0.0.1:5000/cert/add

# Add a route for host `ssl`
curl -v -d @examples/route-ssl.json http://127.0.0.1:5000/route/add

# Send a request through the proxy.
curl -v -k --connect-to ::127.0.0.1:4433 https://ssl/get

# Stop the server and clean up the generated files
pkill -INT granite
rm ssl.crt ssl.key ssl.json
```

### Mutual TLS on configuration API

```bash
# Create a self-signed certificate and key for the client and server.
openssl req -x509 -new -nodes -subj "/CN=api" -out api.crt -keyout api.key
openssl req -x509 -new -nodes -subj "/CN=client" -out client.crt -keyout client.key

# Create a configuration file with mutual TLS enabled.
cat << EOF > conf.yaml
api:
  tls: true
  mutual_tls: true
  cert: api.crt
  key: api.key
  client_cert: client.crt
EOF

# Start the server
RUST_LOG=info cargo run -- --daemon --conf conf.yaml

# Add a route
curl -v \
  --cacert api.crt \
  --cert client.crt --key client.key \
  -d @examples/route-forward.json \
  --connect-to ::127.0.0.1:5000 \
  https://api/route/add

# Send a request through the proxy
curl -v --connect-to ::127.0.0.1:8080 http://forward/get

# Stop the server and clean up the generated files
pkill -INT granite
rm api.crt api.key client.crt client.key conf.yaml
```

### Multiple origins (one bad, one good)

```bash
# Start the server
RUST_LOG=info cargo run -- --daemon

# Add a route with two origins, one of which is unreachable.  The bad origin has a much higher
# weight than the good origin, so the proxy is more likely to try the bad origin first.
curl -v -d @examples/route-badorigin.json http://127.0.0.1:5000/route/add

# Send a request through the proxy.  The proxy will attempt to connect to the bad origin.
# When that fails, it will mark the bad origin as down for 10 seconds, and try the good origin,
# which will succeed.
curl -v --connect-to ::127.0.0.1:8080 http://badorigin/get

# Send the same request again.  If it's been less than 10 seconds since the last request, the proxy
# will only try the good origin because the bad origin is still marked down.
curl -v --connect-to ::127.0.0.1:8080 http://badorigin/get

# Stop the server
pkill -INT granite
```

## Internals

See the [architecture](docs/architecture.md) documentation.
