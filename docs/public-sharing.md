# Public sharing review

Reviewed 22 October 2026, starting from release 0.2.2.

- Gitleaks 8.30.1 scanned the six existing commits and current app files with no findings. A separate pattern scan of 68 historical Git blobs found no private LAN addresses, MAC addresses, private key material, known credential prefixes or private setup paths in text. This is a bounded review, not a guarantee that no vulnerability or sensitive data exists.
- Existing public captures used original demo icons. The refreshed gallery uses browser demo apps and an original AI-generated scenic still. No personal TV captures, installed app artwork, Apple video frames, private profiles or helper addresses are published.
- GitHub secret scanning and push protection were verified enabled. Private vulnerability reporting was enabled. CI now scans full Git history with a checksum-verified Gitleaks binary and checks tracked filenames/text for private runtime artifacts and device details. Workflow actions are pinned to commit hashes and use read-only repository permissions.
- Only `app/` enters the IPK. The showcase backdrop, screenshots, docs and local presentation server are outside it. The gallery does not add a wallpaper or screenshots to the production TV package.
- The public author identity is the existing GitHub profile and GitHub noreply commit address. No history rewrite was required by this review.

For future changes, follow [CONTRIBUTING.md](../CONTRIBUTING.md) and [SECURITY.md](../SECURITY.md). Scan the entire history after changes, and visually inspect every screenshot. Keep recordings, machine configuration, key material and storage exports outside the repository.
