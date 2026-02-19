import PocketBase from 'pocketbase';

// CONFIGURATION
// Default to 127.0.0.1:8090 unless specified
let PB_URL = 'http://127.0.0.1:8090';

// Arguments:
// 1. URL (Optional, defaults to local)
// 2. Email
// 3. Password
if (process.argv.length === 5) {
    PB_URL = process.argv[2];
}

const ADMIN_EMAIL = process.argv[process.argv.length - 2];
const ADMIN_PASS = process.argv[process.argv.length - 1];

if (!ADMIN_EMAIL || !ADMIN_PASS) {
    console.error("Usage: node init_pb_schema.js [URL] <admin_email> <admin_pass>");
    console.error("Example: node init_pb_schema.js https://onyx.omaritani.dev admin@onyx.dev pass123");
    process.exit(1);
}

const pb = new PocketBase(PB_URL);

async function main() {
    try {
        console.log(`Connecting to ${PB_URL}...`);
        await pb.admins.authWithPassword(ADMIN_EMAIL, ADMIN_PASS);
        console.log("✅ Authenticated as Admin.");

        // 1. UPDATE USERS COLLECTION (The ZK Fortress)
        console.log("🔒 Configuring 'users' collection for Zero-Knowledge...");
        const usersCollection = await pb.collections.getOne('users');

        // Update Schema
        await pb.collections.update(usersCollection.id, {
            schema: [
                // Real Email is BLIND INDEX (Hash)
                // We use the system 'email' field for the Blind Index.

                // Encrypted Blobs
                { name: 'email_ciphertext', type: 'text', required: true, presentable: false }, // AES-GCM
                { name: 'name_ciphertext', type: 'text', required: false, presentable: false },
                { name: 'avatar_ciphertext', type: 'text', required: false, presentable: false },

                // Crypto Vars
                { name: 'enc_salt', type: 'text', required: true, presentable: false }, // For KDF
                { name: 'key_wrapped_rk', type: 'text', required: true, presentable: false }, // Wrapped Master Key
                { name: 'recovery_hash', type: 'text', required: true, presentable: false }, // Argon2 of Recovery Key

                // Device Security
                { name: 'hwid_ciphertext', type: 'text', required: false, presentable: false },
            ],
            // STRICT API RULES
            // Only Owner can see their ciphertext. Public can create (Signup).
            listRule: "id = @request.auth.id",
            viewRule: "id = @request.auth.id",
            createRule: "", // Public signup
            updateRule: "id = @request.auth.id",
            deleteRule: "id = @request.auth.id",
        });
        console.log("✅ 'users' collection hardened.");

        // 2. CREATE PASSKEYS COLLECTION
        try {
            await pb.collections.create({
                name: 'passkeys',
                type: 'base',
                schema: [
                    { name: 'user', type: 'relation', collectionId: usersCollection.id, cascadeDelete: true, maxSelect: 1, required: true },
                    { name: 'credential_id', type: 'text', required: true, presentable: false }, // Public Cred ID
                    { name: 'public_key', type: 'text', required: true, presentable: false }, // COSE Key
                    { name: 'label_ciphertext', type: 'text', required: true, presentable: false }, // Encrypted "iPhone"
                    { name: 'counter', type: 'number', required: true, presentable: false },
                    { name: 'transports', type: 'json', required: false, presentable: false },
                    { name: 'last_used', type: 'date', required: false, presentable: false },
                ],
                listRule: "user = @request.auth.id",
                viewRule: "user = @request.auth.id",
                createRule: "user = @request.auth.id",
                updateRule: "user = @request.auth.id",
                deleteRule: "user = @request.auth.id",
                indexes: ["CREATE UNIQUE INDEX idx_credential_id ON passkeys (credential_id)"]
            });
            console.log("✅ 'passkeys' collection created.");
        } catch (e) {
            console.log("⚠️ 'passkeys' might already exist, skipping...");
        }

        // 3. CREATE AUTH_TOKENS COLLECTION (Stateless OTPs)
        try {
            await pb.collections.create({
                name: 'auth_tokens',
                type: 'base',
                schema: [
                    { name: 'user', type: 'relation', collectionId: usersCollection.id, cascadeDelete: true, maxSelect: 1, required: true },
                    { name: 'token_hash', type: 'text', required: true, presentable: false }, // sha256(token)
                    { name: 'type', type: 'select', options: { values: ['otp', 'magic_link', 'recovery'] }, required: true },
                    { name: 'expires_at', type: 'date', required: true },
                    { name: 'used', type: 'bool', required: false },
                ],
                // Only System/Server creates these. Users can't list them (Blind).
                listRule: null,
                viewRule: null,
                createRule: null,
                updateRule: null,
                deleteRule: null,
            });
            console.log("✅ 'auth_tokens' collection created.");
        } catch (e) {
            console.log("⚠️ 'auth_tokens' might already exist, skipping...");
        }

        console.log("\n🚀 ZERO-KNOWLEDGE SCHEMA DEPLOYED SUCCESSFULLY!");

    } catch (err) {
        console.error("❌ Failed:", err.data || err.message);
    }
}

main();
