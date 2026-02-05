# 🔄 Syncing Onyx (Bring Your Own Cloud)

Onyx is a **Local-First** application. This means your notes live on your device, not on our servers. This ensures:
1.  **Privacy**: We can't see your data even if we wanted to.
2.  **Ownership**: You own the files (standard `.json` / `.md`).
3.  **Speed**: Zero network latency while typing.

However, to sync between devices (e.g., PC to Laptop to Mobile), you need to bridge them. Here are the recommended ways.

## Method 1: Git Sync (Recommended for Pros) 👩‍💻
Since Onyx notes are just text files, Git is the most powerful way to sync them with infinite version history.

### Setup
1.  Create a **Private Repository** on GitHub/GitLab.
2.  Open your Terminal in the Onyx Notes folder (usually `Documents/Onyx Development/onyx_data` or wherever you save files).
3.  Initialize Git:
    ```bash
    git init
    git remote add origin <your-repo-url>
    ```
4.  **Sync Routine**:
    *   **Save**: `git add . && git commit -m "Backup" && git push`
    *   **Load**: `git pull`

> **Note**: In the future, Onyx aims to automate this "Push/Pull" process directly in the app settings!

## Method 2: Syncthing (P2P - Free & Real-Time-ish) 🤝
If you want privacy without servers, **Syncthing** connects your devices directly (Peer-to-Peer).

1.  Current Best Option for "Free Real-Time".
2.  Install [Syncthing](https://syncthing.net/) on your PC and Android/Laptop.
3.  Point both devices to your Onyx Data folder.
4.  **Magic**: When you verify a file on PC, Syncthing instantly pushes the bits to your Phone over your local WiFi (or encrypted internet relay).
5.  **Pros**: Unlimited data, fast, no cloud giants.
6.  **Cons**: Both devices must be ON to sync.

## Method 3: Cloud Drives (Easiest) ☁️
Simply move your Onyx storage folder into:
*   Google Drive
*   OneDrive
*   Dropbox
*   iCloud

The app will read/write files there, and the cloud provider handles the syncing.

---
## 🚧 Future: Onyx Cloud?
We are researching an encrypted, real-time sync solution (using CRDTs and P2P Relays) that maintains the "Privacy First" promise while offering the convenience of Notion-like syncing. Stay tuned!
