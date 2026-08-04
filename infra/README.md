# Infra

Versioned copies of everything that lives on the droplet (206.223.241.165),
so the deploy path is reconstructable from the repo alone.

- `post-receive` — the bare repo's git hook (`~/git/unseeing.git/hooks/`).
  Checks out each push to `main` and runs the repo's own `ci/pipeline.sh`.
  User-owned; update by copying over ssh, no sudo needed.
- `nginx-unseeing.conf` — the nginx site (`/etc/nginx/sites-available/unseeing`).
  Root-owned; apply with the commands in the file's header (needs sudo).

## Droplet layout

- `~/bin/godot` — headless Godot (version pinned by `.godot-version`)
- `~/.local/share/godot/export_templates/<ver>/` — web export templates only
- `~/git/unseeing.git` — bare repo (`git push production main` deploys)
- `~/ci/work` — pipeline checkout of the last pushed commit
- `/var/www/unseeing` — the served build (user-writable, deployed by the pipeline)
- chromium — used by the pipeline's browser smoke test

## Crash beacon

The web shell reports JS errors to `/err?b=<build>&m=<message>`; nginx logs
them to `/var/log/nginx/unseeing-err.log`. Check it with:
`ssh vpn 'tail /var/log/nginx/unseeing-err.log'`
