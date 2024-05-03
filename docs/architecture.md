TODO: Describe the architecture of the project.  Include a diagram and briefly
explain each component.


```mermaid
flowchart TD
  subgraph "Proxy Service"
  Proxy
  CertProvider
  end
  subgraph "ConfigApi Service"
  ConfigApi
  end
  ConfigApi --> RouteStore
  ConfigApi --> CertStore
  Proxy --> RouteStore
  CertProvider --> CertStore
```
