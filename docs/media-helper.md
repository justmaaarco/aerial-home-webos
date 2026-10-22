# Optional local media helper

Apple's newer Mac aerial files use 240 fps timing. That source format did not start hardware decoding in our LG B4 test. The helper retimes it to approximately 29.97 fps with FFmpeg **stream copy**: no scaling, no pixel re-encoding, no bit-depth reduction and no fabricated HDR. The source's colour metadata remains intact.

Only these high-frame-rate Mac files use the helper. Native 4K Dolby Vision / SDR TV files stream directly from Apple. You can run the helper on a Raspberry Pi or another LAN machine with Node.js 22+, FFmpeg and ffprobe on PATH:

```sh
git clone https://github.com/justmaaarco/aerial-home-webos.git
cd aerial-home-webos
AERIAL_BIND=0.0.0.0 AERIAL_PORT=8794 node tools/media-server.cjs
```

The default bind address is loopback; set `AERIAL_BIND` explicitly for LAN playback. In Aerial Home, open Aerials → Compatibility helper, enter the machine's HTTP LAN address and port, then Save. The machine must remain available for those clips. The launcher and native TV aerials work without it.

## Resource behavior

- One preparation job runs at a time; the queue is limited to three waiting jobs.
- The first play may need several minutes to fetch and retime the video. The existing aerial keeps playing during preparation.
- Ready files occupy at most 3072 MiB by default; older inactive files are evicted. An in-progress temporary file can occupy up to another cache-limit amount.
- `AERIAL_CACHE` changes the cache directory; `AERIAL_CACHE_MB` changes the limit.
- Seeking/range requests and HEAD are supported for native TV playback. Currently streamed files are protected from normal cache eviction.
- ffprobe checks 3840×2160 HEVC, 10-bit pixels and a TV-compatible frame rate before a file is marked ready.
- Preparation uses stream copy, not a CPU-heavy video encoder. A low-priority service is appropriate on a Pi shared with other work.

No videos are bundled or uploaded to GitHub. The cache belongs to the local user. The API accepts known catalogue IDs, never arbitrary source URLs, commands or file paths. It exposes public media and preparation status, not TV control or credentials. Keep it on your LAN.

For a persistent service, run the command under your system's service manager as an ordinary user, set a writable cache path, and limit memory/CPU as appropriate. Stop the service to disable it; native Apple streams continue to work.
