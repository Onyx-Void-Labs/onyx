# Deploying Self-Hosted PocketBase (The Zero-Log Way)

This guide will walk you through wiping the current VPS setup and installing the new "Privacy-First" PocketBase stack.

## Prerequisites
- VPS IP: `104.168.82.148`
- User: `onyx` (or root)
- Domain: You need a domain pointing to this IP (e.g., `onyx.omaritani.dev`).
- **Tools**: Ensure you have `ssh` and `scp` (or Putty/WinSCP) installed.

## Step 1: Prepare the VPS (Clean Slate)

SSH into your VPS:
```powershell
ssh onyx@104.168.82.148
```

Stop any running containers and clean up:
```bash
# Go to your deployment folder (if it exists)
cd ~/onyx/deploy || mkdir -p ~/onyx/deploy

# Stop everything
docker compose down

# OPTIONAL: Nuke everything (WARNING: DELETES ALL DATA)
# docker system prune -a --volumes
# rm -rf ~/onyx
```

## Step 2: Upload the New Configuration

From your **local machine** (PowerShell), upload the new files:

```powershell
# 1. Upload Docker Compose
scp .\deploy\docker-compose.pb.yml onyx@104.168.82.148:~/onyx/deploy/docker-compose.yml

# 2. Upload Caddyfile
scp .\deploy\Caddyfile.pb onyx@104.168.82.148:~/onyx/deploy/Caddyfile

# 3. Upload Hocuspocus Server Code (Since we changed it)
# We need to send the whole folder so Docker can build it
scp -r .\hocuspocus-server onyx@104.168.82.148:~/onyx/
```

## Step 3: Start the Privacy Stack

Back on your **VPS**:

```bash
cd ~/onyx/deploy

# Build and Start (This will build the new Hocuspocus image)
docker compose up -d --build
```

## Step 4: Initial Setup

1.  Open your browser to `https://onyx.omaritani.dev/_/` (The Admin UI).
2.  Create your **Admin Account**.
3.  **Critical Security Step**:
    - Go to **Settings** -> **Logs**.
    - Set "Retention days" to `0` (Zero-Log Policy).
    - Uncheck "Log user IP" and "Log user agent".

### Troubleshooting

**Hocuspocus `MODULE_NOT_FOUND`**:
If you see `Error: Cannot find module '/app/index.js'`, ensure your `docker-compose.pb.yml` mounts the volume to `/app/data` (not `/app`) and that `index.js` writes to `./data/hocuspocus.db`.

**Check Logs**:
```bash
docker logs onyx-pocketbase
docker logs onyx-hocuspocus
docker logs onyx-caddy
```

## Step 5: Verify "Zero-Log"

Check the running logs to make sure Caddy is silent:
```bash
docker logs onyx-caddy
```

You should also see Hocuspocus running:
```bash
docker logs onyx-hocuspocus
# Output: Hocuspocus Server running on port 1234, connected to http://pocketbase:8090
```

---

### What's Next?
Now that the backend is live:
1.  **Update Client**: We will replace `supabase.ts` with `pocketbase.ts` in your Tauri app.
2.  **Update Sync**: We will point the `SyncContext` to `wss://onyx.omaritani.dev/ws`.
