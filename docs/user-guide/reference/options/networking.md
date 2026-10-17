# Networking options

Open **File → Options… → Networking** to configure requests to online tag sources and the update service.

!!! tip "New to this?"
    Keep the system proxy choice unless the network requires a different route. A longer timeout helps a slow server finish; it does not fix an incorrect address or an authentication failure.

## Timeout

**Timeout** defaults to **30 seconds**. Enter a whole number from **5** to **300**. A request that takes too long is abandoned and reported as a failure. This is a request timeout, not a promise that a multi-request search will finish within that total time. Establishing the connection has a separate ceiling of 10 seconds, or the chosen timeout if shorter.

## Proxy

| Choice | Behavior |
|---|---|
| **Use the system proxy settings** | Default. Let the HTTP client use system/environment proxy configuration, including applicable `HTTP_PROXY`/`HTTPS_PROXY` values. |
| **No proxy** | Make a direct connection through the client's no-proxy mode. Use when inherited proxy settings are inappropriate. |
| **Use this HTTP proxy** | Use the address entered below for the app's HTTP requests. |

For **Proxy address**, enter a host and port such as `proxy.example.test:8080`, or an HTTP/HTTPS URL such as `http://proxy.example.test:8080`. A missing scheme is interpreted as `http://`. Paths beyond `/`, query strings, unsupported schemes, and embedded user names/passwords are rejected.

Proxy authentication is not supported. Ordinary options can be stored and exported as plain text, so do not attempt to insert credentials into the address. Ask the network administrator for an appropriate system-proxy route if authentication is required.

These settings govern TagGroove's HTTP client; a web browser or external [tool](tools.md) has its own network configuration. Choosing a proxy here does not change the operating system's settings.

## Related

- [Tag Sources](tag-sources.md)
- [Updates](updates.md)
- [Tools](tools.md)

<!-- verified: src/general-options.ts NETWORK_DEFAULTS, TIMEOUT_RANGE, networkProblem; src-tauri/src/settings.rs Network, ProxyMode, proxy_address; src-tauri/src HTTP-client configuration. -->
