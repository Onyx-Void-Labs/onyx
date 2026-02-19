# Deployment Guide (VPS) - Phase 2 (Hocuspocus + Postgres)

This guide explains how to securely deploy the **Onyx Sync Stack** (Hocuspocus Server, PostgreSQL, Caddy) to your VPS.

## 1. Security First 🛡️
Before you do anything, understand how secrets work:
- **Local `.env`**: Contains `VITE_` keys for the frontend build.
- **VPS `.env`**: Contains sensitive server-side keys (`DB_PASSWORD`, `SUPABASE_SERVICE_ROLE_KEY`).
- **Safety**: `.env` is now in `.gitignore`, so it will **never** be uploaded to GitHub. You must create the `.env` on the VPS manually.

## 2. Prepare the VPS
SSH into your VPS and run these commands to clear old versions.

```bash
# 1. SSH in
ssh root@104.168.82.148

# 2. Stop everything and clean up volumes (Fresh Start)
docker stop $(docker ps -a -q)
docker system prune -a --volumes -f

# 3. Create project directory
mkdir -p ~/onyx-sync
```

## 3. Transfer the Code
From your **LOCAL** machine (where the code is), use `scp` to send the backend folders.

```bash
# Navigate to your project folder locally
cd "C:\Users\omar_\Documents\Onyx Development\onyx"

# Upload the Hocuspocus server and Deploy config
scp -r hocuspocus-server root@104.168.82.148:~/onyx-sync/
scp -r deploy root@104.168.82.148:~/onyx-sync/
```

## 4. Manage Secrets on VPS (Locked Down)
Now, create the secure environment file **directly on the VPS**.

```bash
# Back in VPS SSH session
cd ~/onyx-sync/deploy
nano .env
```

Paste your secrets into the file. **Do not share these!**
```ini
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_SERVICE_ROLE_KEY=your-secret-role-key
DB_PASSWORD=choose_a_strong_password
POSTGRES_DB=onyx_sync
DATABASE_URL=postgres://postgres:choose_a_strong_password@postgres:5432/onyx_sync
```
*Note: Save with `Ctrl+O`, then `Enter`, then exit with `Ctrl+X`.*

**Lock Permissions**:
```bash
chmod 600 .env # Only the owner can read/write this file
```

## 5. Launch the Stack
1. **Update Caddyfile**: Replace `sync.your domain.com` with your real sync domain.
2. **Start Docker**:
   ```bash
   docker-compose up -d --build
   ```

## 6. Update Frontend (Local)
Build your frontend with the new provider URL:
```ini
# .env (local)
VITE_WS_URL=wss://sync.YOUR-REAL-DOMAIN.com
```

## 7. Sending Updates Later
When you change the backend code:
1. Local: `scp -r hocuspocus-server root@...:~/onyx-sync/`
2. VPS: `cd ~/onyx-sync/deploy && docker-compose up -d --build hocuspocus`
