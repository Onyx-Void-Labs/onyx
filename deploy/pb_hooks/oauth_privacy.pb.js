/// <reference path="../pb_data/types.d.ts" />

/**
 * Onyx Overkill Privacy Hook
 * 
 * This hook intercepts OAuth2 logins (Google/Apple) in memory.
 * It hashes the incoming email BEFORE it is ever written to the database.
 * This ensures the plaintext email never touches the disk.
 * 
 * TO INSTALL:
 * Copy this file to your PocketBase `pb_hooks` directory.
 */

onRecordBeforeAuthWithOAuth2Request((e) => {
    // 1. Get the incoming email from the provider (in memory)
    const realEmail = e.oauth2User.email;

    if (realEmail) {
        // 2. Hash it immediately
        // consistent hash so they can log in again via OAuth
        const blindHash = $security.sha256(realEmail);
        const blindEmail = blindHash.substring(0, 15) + "@blind.onyx";

        // 3. Replace the email in the record to be saved
        e.record.set("email", blindEmail);

        // 4. Blind the name and avatar to ensure NO PII is stored
        e.record.set("name", ""); // Clear name
        e.record.set("avatar", ""); // Clear avatar

        // 5. Ensure it's verified
        e.record.set("verified", true);

        // Log for debugging
        console.log("[Onyx Hook] Blinded PII for OAuth user:", blindEmail);
    }
}, "users")
