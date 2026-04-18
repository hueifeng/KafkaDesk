# Security Policy

KafkaDesk is a desktop-first debugging workbench that connects directly to Kafka-compatible infrastructure from an engineer workstation. That makes security behavior part of the product surface, not an afterthought.

## Reporting a Vulnerability

Please **do not** open a public GitHub issue for suspected vulnerabilities.

Until a dedicated security contact is published, use a private maintainer-controlled channel:

- prefer a GitHub private vulnerability report / security advisory if the repository has that feature enabled
- otherwise contact the current maintainer/owner privately before sharing exploit details publicly

When reporting, include:

- affected workflow or feature area
- steps to reproduce
- impact assessment
- whether credentials, certificate material, replay behavior, or cluster connectivity are involved
- the KafkaDesk version/commit and operating system

## Supported Scope

KafkaDesk is still pre-1.0 and does not yet publish formal supported release trains.

For now, security fixes should be assumed to target:

- the current repository `main` branch / latest source state
- the current desktop runtime implementation under `src-tauri/`

Older local checkouts, forks, or unpublished builds should not be assumed to receive coordinated backports.

## Runtime Security Caveats

### Direct cluster access from the engineer machine

KafkaDesk is designed to connect to Kafka-compatible systems directly from a local machine. That means:

- local VPN/network access may determine whether secured clusters are reachable
- workstation certificate stores, filesystem permissions, and keychain/keyring state matter
- a user can only be as secure as the machine and local account running KafkaDesk

### Secrets and credentials

- do not commit real broker credentials, schema registry credentials, private keys, or production certificate material
- prefer keyring-backed `credentialRef` flows over plaintext secret storage
- treat test secrets and generated cert material as disposable local development artifacts only

### Replay safety

Replay can publish back to Kafka brokers. Use caution with any environment that is not explicitly sandboxed or otherwise approved for controlled replay.

Before using live replay paths, verify:

- the target cluster/environment is the intended one
- delivery timeout/retry behavior matches your operational expectations
- credentials and TLS material map to the correct environment

### TLS / SSL runtime assumptions

The repository currently builds Kafka TLS support through `rdkafka` with vendored OpenSSL in supported build environments. Downstream consumers that remove or replace that support can change secured-cluster behavior.

If a downstream build strips SSL/OpenSSL support, KafkaDesk should fail truthfully rather than reporting a secured path as ready.

## Secure Contribution Guidelines

When changing security-sensitive code or docs:

- keep validation and failure states truthful
- avoid adding fallback behavior that silently weakens auth or TLS expectations
- update tests and operator-facing docs together
- document any risk acceptance in the pull request, decision record, or other maintainer-visible review artifact
