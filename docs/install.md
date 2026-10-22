# Installation and recovery

Build the IPK with `node tools/package.cjs`. Compare its checksum with the published `.sha256` file if downloading a release.

Use webOS Dev Manager's package installer with a TV connection that is already configured, or the LG CLI:

```sh
ares-install --device YOUR_TV dist/io.github.justmaaarco.aerialhome_0.1.0_all.ipk
ares-launch --device YOUR_TV io.github.justmaaarco.aerialhome
```

The current inventory and local artwork adapter also needs an already-rooted TV and Homebrew Channel. The app does not root the TV, enable SSH, install keys or install boot hooks. The browser preview works without those services.

For a known rooted TV, Homebrew Channel can install a reachable IPK with its SHA-256 checksum through `org.webosbrew.hbchannel.service/install`. See the [Homebrew command reference](https://www.webosbrew.org/pages/commands-cheatsheet). Do not expose the TV's Luna/SSH services to the internet.

To remove the app, uninstall Aerial Home through your app/package manager, or:

```sh
ares-install --device YOUR_TV --remove io.github.justmaaarco.aerialhome
```

The app stores its own preferences under `aerial-home-v1` in localStorage. It does not change picture settings, firmware, existing launchers or boot configuration. If a separate startup integration has been configured manually, revert that integration before removing its target app.

No Pi process is needed after installation. Video is streamed by the TV. Loading a different app releases the background video when webOS reports that the launcher is hidden.

Integrations can launch a particular panel using app launch parameters, for example `{"screen":"controls"}`. Supported screen values are `controls`, `settings`, `inputs` and `search`.
