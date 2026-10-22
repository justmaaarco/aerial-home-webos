# Security and privacy

Report vulnerabilities privately using [Report a vulnerability](https://github.com/justmaaarco/aerial-home-webos/security/advisories/new). Include the affected version, webOS version, reproduction steps and impact. Remove addresses, keys, personal app inventories and account information. Do not put credentials or exploit details in public issues.

The current alpha is maintained on the latest tagged release. Fix availability depends on reproduction and device access; no response-time or compatibility guarantee is offered.

## Trust boundaries

- The TV adapter requires an already-rooted TV and Homebrew Channel. It issues a small allowlist of local commands for inventory, capabilities and validated app artwork. Root access increases the impact of mistakes; review changes to the adapter before installing.
- App titles, IDs and asset paths are treated as data. Do not add arbitrary shell commands, remote command endpoints or permissive path readers.
- The optional media helper defaults to loopback. Explicit LAN binding is for trusted local networks; it has no authentication, and clients can request preparation of known catalogue entries. Do not forward its port to the internet.
- Apple media URLs currently use HTTP and are public, credential-free resources. This offers no transport integrity against a hostile network. The helper's resource limits and format probes are not a sandbox for FFmpeg; keep FFmpeg/Node updated and use a restricted ordinary service account.
- App preferences, cached artwork and helper addresses stay on the local device. The app has no analytics or telemetry endpoint. The helper contacts Apple for selected media.

## Before sharing

Run the tests, `node tools/check-public.cjs`, and Gitleaks against the full Git history. Inspect screenshots for device names, profiles, addresses and installed-app artwork. Use the supplied browser demo for public screenshots. Ignore rules are preventive helpers, not a guarantee that a file is safe to commit.
