# Contributing

Use Node.js 22 or newer. No package installation is needed for the app, tests or packager.

```sh
node --test test/*.test.cjs
node tools/check-public.cjs
node tools/package.cjs
```

Describe the behavior changed, why, and what you tested. Separate browser checks from physical-TV checks and include webOS version for hardware findings. Avoid claims of pixel parity, supported HDR formats or performance gains without matching evidence.

The production package is built only from `app/`. Keep screenshot backdrops, generated art and documentation outside that directory. Use `node tools/showcase.cjs` for safe demo captures; it never connects to a TV.

Do not commit credentials, SSH material, LAN addresses, MAC addresses, actual device inventories, browser storage exports or private captures. Use placeholder names such as `YOUR_TV` in instructions. Public screenshots should contain only demo apps and the original demo backdrop. Preserve MIT and Inter/OFL notices; Apple videos and installed third-party app artwork are not covered by this repository's license.

For a security issue, follow [SECURITY.md](SECURITY.md).
