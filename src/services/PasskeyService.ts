
export class PasskeyService {

    // --- UTILS: Base64URL Encoding/Decoding ---

    // Encode ArrayBuffer to Base64URL string
    static bufferToBase64URL(buffer: ArrayBuffer): string {
        const bytes = new Uint8Array(buffer);
        let str = '';
        for (const charCode of bytes) {
            str += String.fromCharCode(charCode);
        }
        const users = btoa(str);
        return users.replace(/\+/g, '-').replace(/\//g, '_').replace(/=/g, '');
    }

    // Decode Base64URL string to Uint8Array
    static base64URLToBuffer(base64url: string): Uint8Array {
        const padding = '='.repeat((4 - base64url.length % 4) % 4);
        const base64 = (base64url + padding).replace(/-/g, '+').replace(/_/g, '/');
        const raw = atob(base64);
        const output = new Uint8Array(raw.length);
        for (let i = 0; i < raw.length; i++) {
            output[i] = raw.charCodeAt(i);
        }
        return output;
    }

    // --- REGISTRATION ---

    /**
     * Registers a new Passkey (WebAuthn Credential) for the user.
     * @param username Display name for the credential (e.g. "Omar")
     * @param userId Stable user ID (e.g. "u_8x3k2n")
     */
    static async register(username: string, userId: string): Promise<any> {
        console.log("[Passkey] Starting Registration...", { username, userId });

        // 1. Challenge
        const challenge = new Uint8Array(32);
        window.crypto.getRandomValues(challenge);

        // 2. User Info
        const userHandle = new TextEncoder().encode(userId);

        // 3. Create Credential Options
        // We relax "authenticatorAttachment" to allow fallback if platform fails,
        // but prefer "platform" for built-in ease.
        const publicKeyCredentialCreationOptions: PublicKeyCredentialCreationOptions = {
            challenge: challenge,
            rp: {
                name: "Onyx",
                id: window.location.hostname // Crucial: Must match current domain
            },
            user: {
                id: userHandle,
                name: username,
                displayName: username,
            },
            pubKeyCredParams: [
                { alg: -7, type: "public-key" }, // ES256
                { alg: -257, type: "public-key" }, // RS256
            ],
            authenticatorSelection: {
                // "platform" forces Windows Hello / Touch ID.
                // If this fails on some setups, removing it allows YubiKeys etc.
                // We keep it "platform" as verified preferred, but you can remove to debug.
                authenticatorAttachment: "platform",
                userVerification: "required", // Force PIN/Biometrics
                residentKey: "required", // Discoverable Credential
            },
            timeout: 60000,
            attestation: "none",
        };

        console.log("[Passkey] Options:", publicKeyCredentialCreationOptions);

        try {
            // 4. Create Credential
            const credential = await navigator.credentials.create({
                publicKey: publicKeyCredentialCreationOptions,
            }) as PublicKeyCredential;

            if (!credential) throw new Error("Credential creation returned null.");

            console.log("[Passkey] Created:", credential);

            const response = credential.response as AuthenticatorAttestationResponse;
            const transports = typeof response.getTransports === 'function' ? response.getTransports() : [];

            return {
                id: credential.id,
                rawId: this.bufferToBase64URL(credential.rawId),
                response: {
                    clientDataJSON: this.bufferToBase64URL(response.clientDataJSON),
                    attestationObject: this.bufferToBase64URL(response.attestationObject),
                    transports: transports
                },
                type: credential.type,
                user_id: userId,
            };
        } catch (err) {
            console.error("[Passkey] Registration Failed:", err);
            // Fallback suggestion logic could go here
            throw err;
        }
    }

    // --- AUTHENTICATION ---

    /**
     * Authenticates using a Passkey.
     * If no userId is provided, it attempts to "Discover" the user (Resident Key).
     */
    static async authenticate(_challengeStr?: string): Promise<any> {
        console.log("[Passkey] Starting Authentication...");

        const challenge = new Uint8Array(32);
        window.crypto.getRandomValues(challenge);

        const publicKeyCredentialRequestOptions: PublicKeyCredentialRequestOptions = {
            challenge: challenge,
            rpId: window.location.hostname,
            userVerification: "required",
            // allowCredentials: [] // Empty = Discoverable Credential (Resident Key)
        };

        console.log("[Passkey] Auth Options:", publicKeyCredentialRequestOptions);

        try {
            const credential = await navigator.credentials.get({
                publicKey: publicKeyCredentialRequestOptions,
            }) as PublicKeyCredential;

            if (!credential) throw new Error("Authentication returned null.");

            console.log("[Passkey] Authenticated:", credential);

            return {
                id: credential.id,
                rawId: this.bufferToBase64URL(credential.rawId),
                response: {
                    clientDataJSON: this.bufferToBase64URL((credential.response as AuthenticatorAssertionResponse).clientDataJSON),
                    authenticatorData: this.bufferToBase64URL((credential.response as AuthenticatorAssertionResponse).authenticatorData),
                    signature: this.bufferToBase64URL((credential.response as AuthenticatorAssertionResponse).signature),
                    userHandle: (credential.response as AuthenticatorAssertionResponse).userHandle
                        ? this.bufferToBase64URL((credential.response as AuthenticatorAssertionResponse).userHandle!)
                        : null
                },
                type: credential.type
            };
        } catch (err) {
            console.error("[Passkey] Authentication Failed:", err);
            throw err;
        }
    }
}
