import PocketBase from 'pocketbase';

// Determine PB URL based on environment
// In production, this points to your VPS (proxied through Caddy)
// In development, it points to local PB
const pbUrl = import.meta.env.VITE_POCKETBASE_URL || 'http://127.0.0.1:8090';

export const pb = new PocketBase(pbUrl);

// Optional: specific collection helper
export const usersCollection = pb.collection('users');
