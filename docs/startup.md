# Startup performance

Version 0.2.1 renders cached app metadata first and refreshes it in the background. Fresh app inventory does not wait for HDMI input discovery, HDR capabilities or the one-time Luma favourite import. Video format selection still checks capabilities independently. An unchanged Home selection is not rebuilt after discovery, preserving its current tiles and focus.

Artwork is read from an asynchronous on-TV IndexedDB cache before requesting it through Homebrew Channel. It stores at most 24 records, each limited to 400,000 data-URL characters. Records are tied to the app ID and its inventory asset path; changed paths or seven-day expiry require fresh artwork. Metadata also expires after seven days. These caches are private to the TV, contain no Apple video files and are never uploaded by the app. The private caches do not form part of the public package or screenshots in this repository.

If storage is unavailable, corrupt, expired or full, the launcher falls back to normal discovery and artwork loading. Without a valid metadata cache, the first Home still waits for app inventory. A cached app that has just been removed can briefly appear until the refresh completes; failed launches report a TV error. Cache capacity can evict icons, so a large All apps view may still fetch artwork.

## Measurement method

On the development LG B4 / webOS 9.2.4, three fresh app-process launches of 0.2.0 were compared with three repeat launches of 0.2.1 with populated caches. A temporary measurement harness observed the grid and first image, then recorded the second animation-frame callback after their availability. Timings start at page navigation, include web engine/page startup, and exclude the TV's standby wake before navigation. The harness was removed and the installed source hash restored after each series.

These are software scheduling measurements, not input-to-photon readings or a guarantee for every firmware. Aerials are shuffled, so network/video startup varies between clips. No consistent video-start improvement or menu-latency fix is claimed. Maximum compatible quality selection and moving video behind menus are retained.

| Median of three launches | 0.2.0 | 0.2.1 with cache |
| --- | ---: | ---: |
| Home grid inserted | 967 ms | 449 ms |
| Second frame callback after grid | 1,273 ms | 813 ms |
| Second frame callback after first artwork | 1,286 ms | 1,585 ms |
| First video playing event | 1,398 ms | 1,471 ms |

The grid/frame result improved by about 36%. Artwork and video did **not** show a consistent display-time improvement despite confirmed artwork-cache hits; fewer artwork service requests should not be confused with faster image presentation. Results varied substantially between launches. [Individual samples](startup-measurements.json) retain the numeric observations without private inventories, addresses or artwork. Quick Start+ was off when these launches began; the separate Quick Start+ wake test is not included in this comparison.

`window.AerialStartup` holds local, per-launch diagnostic timestamps for script entry, first Home DOM/frame callbacks, first icon, inventory/capabilities and first video playing event. It is not transmitted or persisted by the production app. An app launch service success reply only acknowledges the request and is separate from these measurements.
