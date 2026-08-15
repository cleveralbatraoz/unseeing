# Infra

Versioned copies of everything that lives on the droplet (206.223.241.165),
so the deploy path is reconstructable from the repo alone.

- `post-receive` — the bare repo's git hook (`~/git/unseeing.git/hooks/`).
  Checks out each push to `main`, plus one-shot `deploy-retry/*` refs when
  `production/main` already names the requested commit, and runs the repo's
  own `ci/pipeline.sh`. Retry refs are deleted after either outcome.
  User-owned; update by copying over ssh, no sudo needed.
- `nginx-unseeing.conf` — the nginx site (`/etc/nginx/sites-available/unseeing`).
  Root-owned; apply with the commands in the file's header (needs sudo).
  Requires the `libnginx-mod-http-brotli-static` package for its
  `brotli_static` directive — install it BEFORE applying the file, or nginx
  refuses to start on an unknown directive.

## Droplet layout

- `~/bin/godot` — headless Godot (version pinned by `.godot-version`)
- `~/.local/share/godot/export_templates/<ver>/` — web export templates only
- `~/git/unseeing.git` — bare repo (`git push production main` deploys)
- `~/ci/work` — pipeline checkout of the last pushed commit
- `~/ci/cargo-target` — cargo's target dir, symlinked in as `work/rust/target`
  because the work tree is wiped every push. Holds the cross-built cores
  `deploy.sh` seeds (this box cannot compile godot-core in 1.8 GB) plus the
  `core.commit` stamp the pipeline checks against the pushed sha, so a failed
  upload cannot leave the previous deploy's binaries in play
- `/var/www/unseeing` — the served build (user-writable, deployed by the pipeline)
- chromium — used by the pipeline's browser smoke test
- `brotli` + `libnginx-mod-http-brotli-static` — the pipeline precompresses
  every shipped artifact with both gzip and brotli; nginx serves whichever
  the client accepts. On the 44 MB wasm that is 10.5 MB gzip vs 7.1 MB
  brotli, for 85 s of single-core deploy time

## Crash beacon

The web shell reports JS errors to `/err?b=<build>&m=<message>`; nginx logs
them to `/var/log/nginx/unseeing-err.log`. Check it with:
`ssh vpn 'tail /var/log/nginx/unseeing-err.log'`
