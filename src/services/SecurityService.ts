
// SecurityService.ts
// Handles all Client-Side Encryption operations using Web Crypto API.

const ALGORITHM = 'AES-GCM';
const KEY_DERIVATION = 'PBKDF2';
const HASH = 'SHA-256';
const ITERATIONS = 100000; // Strong standard
const SALT_LENGTH = 16;
const IV_LENGTH = 12; // Standard for GCM
import * as bip39 from 'bip39';
import { Buffer } from 'buffer';

// Ensure Buffer is available
declare global {
    interface Window {
        Buffer: any;
    }
}

if (typeof window !== 'undefined') {
    window.Buffer = window.Buffer || Buffer;
}

// 0. Deterministic Identity Hashing (Zero-Knowledge Identity)
// Accepts a username (or any identity string) and returns a pseudonymous email for PocketBase auth.
export async function hashIdentity(identity: string): Promise<string> {
    const pepper = import.meta.env.VITE_EMAIL_PEPPER;

    if (!pepper) {
        console.error("CRITICAL: VITE_EMAIL_PEPPER is missing!");
        throw new Error("Security Error: Identity Pepper is missing.");
    }

    const normalizedIdentity = identity.trim().toLowerCase() + pepper;
    const encoder = new TextEncoder();
    const data = encoder.encode(normalizedIdentity);
    const hashBuffer = await window.crypto.subtle.digest('SHA-256', data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    const hashHex = hashArray.map(b => b.toString(16).padStart(2, '0')).join('');

    // Return a pseudonymous email (PocketBase requires email format)
    return `${hashHex}@onyx.internal`;
}

// Backward-compatible alias
export const hashEmail = hashIdentity;

export interface EncryptedNote {
    iv: string;   // Base64
    salt: string; // Base64
    data: string; // Base64 (Ciphertext)
}

// Helper: Base64 to ArrayBuffer (actually using Hex for simplicity in storage, strictly speaking)
// Let's stick to true Base64 for efficiency? actually Hex is safer for JSON sometimes. 
// Let's use Base64 for standard compatibility.

function arrayBufferToBase64(buffer: ArrayBuffer): string {
    let binary = '';
    const bytes = new Uint8Array(buffer);
    const len = bytes.byteLength;
    for (let i = 0; i < len; i++) {
        binary += String.fromCharCode(bytes[i]);
    }
    return window.btoa(binary);
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
    const binary_string = window.atob(base64);
    const len = binary_string.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
        bytes[i] = binary_string.charCodeAt(i);
    }
    return bytes.buffer;
}

// 1. Derive Key from Password (or Mnemonic)
async function deriveKey(secret: string, salt: Uint8Array): Promise<CryptoKey> {
    const textEncoder = new TextEncoder();
    const secretBuffer = textEncoder.encode(secret);

    const importedKey = await window.crypto.subtle.importKey(
        'raw',
        secretBuffer,
        KEY_DERIVATION,
        false,
        ['deriveKey']
    );

    return window.crypto.subtle.deriveKey(
        {
            name: KEY_DERIVATION,
            salt: salt as any,
            iterations: ITERATIONS,
            hash: HASH,
        },
        importedKey,
        { name: ALGORITHM, length: 256 },
        false,
        ['encrypt', 'decrypt', 'wrapKey', 'unwrapKey']
    );
}

// Master Key Management
export const MasterKeyService = {
    generateMasterKey(): string {
        const key = window.crypto.getRandomValues(new Uint8Array(32));
        return arrayBufferToBase64(key.buffer);
    },

    generateSalt(): string {
        const salt = window.crypto.getRandomValues(new Uint8Array(SALT_LENGTH));
        return arrayBufferToBase64(salt.buffer);
    },

    generateRecoveryPhrase(): string {
        return bip39.generateMnemonic();
    },

    async hashString(content: string): Promise<string> {
        const encoder = new TextEncoder();
        const data = encoder.encode(content);
        const hashBuffer = await window.crypto.subtle.digest('SHA-256', data);
        return arrayBufferToBase64(hashBuffer);
    },

    async wrapKey(masterKeyBase64: string, passwordOrMnemonic: string, saltBase64: string): Promise<string> {
        const salt = new Uint8Array(base64ToArrayBuffer(saltBase64));
        const wrappingKey = await deriveKey(passwordOrMnemonic, salt);
        const masterKeyBuffer = base64ToArrayBuffer(masterKeyBase64);

        // We need to import the master key first to wrap it? 
        // Actually, we can just encrypt the raw bytes as data.
        // Wrapping usually implies wrapping a CryptoKey object.
        // Simple approach: Encrypt the MK bytes as if it were a note.
        // We use a fixed IV for the key wrapper? No, random IV is better.
        // But we need to store the IV. 
        // Let's return JSON of { iv, ciphertext } base64 encoded.

        const iv = window.crypto.getRandomValues(new Uint8Array(IV_LENGTH));
        const ciphertext = await window.crypto.subtle.encrypt(
            { name: ALGORITHM, iv },
            wrappingKey,
            masterKeyBuffer
        );

        return JSON.stringify({
            iv: arrayBufferToBase64(iv.buffer),
            data: arrayBufferToBase64(ciphertext)
        });
    },

    async unwrapKey(wrappedJson: string, passwordOrMnemonic: string, saltBase64: string): Promise<string> {
        const salt = new Uint8Array(base64ToArrayBuffer(saltBase64));
        const wrappingKey = await deriveKey(passwordOrMnemonic, salt);

        const wrapper = JSON.parse(wrappedJson);
        const iv = new Uint8Array(base64ToArrayBuffer(wrapper.iv));
        const data = base64ToArrayBuffer(wrapper.data);

        try {
            const decrypted = await window.crypto.subtle.decrypt(
                { name: ALGORITHM, iv },
                wrappingKey,
                data
            );
            return arrayBufferToBase64(decrypted);
        } catch (e) {
            throw new Error("Incorrect Key");
        }
    }
};

// 2. Encrypt Text using Master Key
export async function encryptNote(content: string, masterKeyBase64: string): Promise<EncryptedNote> {
    const textEncoder = new TextEncoder();
    const encodedContent = textEncoder.encode(content);

    // We import the Master Key as a CryptoKey
    const masterKeyBuffer = base64ToArrayBuffer(masterKeyBase64);
    const key = await window.crypto.subtle.importKey(
        "raw",
        masterKeyBuffer,
        ALGORITHM,
        false,
        ["encrypt", "decrypt"]
    );

    const iv = window.crypto.getRandomValues(new Uint8Array(IV_LENGTH));

    const ciphertext = await window.crypto.subtle.encrypt(
        {
            name: ALGORITHM,
            iv: iv,
        },
        key,
        encodedContent
    );

    return {
        iv: arrayBufferToBase64(iv.buffer),
        salt: "", // No salt needed per note anymore as we use pre-derived MK, but keeping field for compat or ignore
        data: arrayBufferToBase64(ciphertext),
    };
}

// 3. Decrypt Text using Master Key
export async function decryptNote(encrypted: EncryptedNote, masterKeyBase64: string): Promise<string> {
    try {
        const iv = new Uint8Array(base64ToArrayBuffer(encrypted.iv));
        const data = base64ToArrayBuffer(encrypted.data);

        const masterKeyBuffer = base64ToArrayBuffer(masterKeyBase64);
        const key = await window.crypto.subtle.importKey(
            "raw",
            masterKeyBuffer,
            ALGORITHM,
            false,
            ["encrypt", "decrypt"]
        );

        const decryptedBuffer = await window.crypto.subtle.decrypt(
            {
                name: ALGORITHM,
                iv: iv,
            },
            key,
            data
        );

        const textDecoder = new TextDecoder();
        return textDecoder.decode(decryptedBuffer);
    } catch (e) {
        console.error("Decryption failed:", e);
        throw new Error("Decryption failed. Invalid Key or Data.");
    }
}

// 4. Data/File Encryption Helpers (For Profile/Avatar)

// Encrypts a File/Blob and returns a new File with [IV + Ciphertext]
export async function encryptFile(file: File, masterKeyBase64: string): Promise<File> {
    const arrayBuffer = await file.arrayBuffer();
    const iv = window.crypto.getRandomValues(new Uint8Array(IV_LENGTH));

    const masterKeyBuffer = base64ToArrayBuffer(masterKeyBase64);
    const key = await window.crypto.subtle.importKey(
        "raw",
        masterKeyBuffer,
        ALGORITHM,
        false,
        ["encrypt"]
    );

    const ciphertext = await window.crypto.subtle.encrypt(
        { name: ALGORITHM, iv },
        key,
        arrayBuffer
    );

    // Combine IV + Ciphertext
    const combined = new Uint8Array(iv.byteLength + ciphertext.byteLength);
    combined.set(iv);
    combined.set(new Uint8Array(ciphertext), iv.byteLength);

    return new File([combined], file.name + ".enc", { type: "application/octet-stream" });
}

// Decrypts an ArrayBuffer (from downloaded file) and returns a Blob URL (for <img>)
export async function decryptFile(data: ArrayBuffer, masterKeyBase64: string, type: string = 'image/png'): Promise<string> {
    try {
        // Extract IV
        const iv = new Uint8Array(data.slice(0, IV_LENGTH));
        const ciphertext = data.slice(IV_LENGTH);

        const masterKeyBuffer = base64ToArrayBuffer(masterKeyBase64);
        const key = await window.crypto.subtle.importKey(
            "raw",
            masterKeyBuffer,
            ALGORITHM,
            false,
            ["decrypt"]
        );

        const decrypted = await window.crypto.subtle.decrypt(
            { name: ALGORITHM, iv },
            key,
            ciphertext
        );

        const blob = new Blob([decrypted], { type });
        return URL.createObjectURL(blob);
    } catch (e) {
        console.error("File decryption failed", e);
        throw new Error("Failed to decrypt file.");
    }
}

// Encrypt string to single JSON string (for simple fields like Name)
export async function encryptData(content: string, masterKeyBase64: string): Promise<string> {
    const encrypted = await encryptNote(content, masterKeyBase64);
    return JSON.stringify(encrypted);
}

// Decrypt JSON string to content
export async function decryptData(jsonString: string, masterKeyBase64: string): Promise<string> {
    try {
        // Check if it's actually JSON/Encrypted
        if (!jsonString.startsWith('{')) return jsonString; // Fallback for legacy/plain text

        const encrypted = JSON.parse(jsonString) as EncryptedNote;
        if (!encrypted.iv || !encrypted.data) return jsonString; // Invalid format

        return await decryptNote(encrypted, masterKeyBase64);
    } catch (e) {
        return jsonString; // Fallback if parse fails
    }
}

// 5. Key Rotation (Re-encrypt Master Key with NEW Recovery Phrase)
export const KeyRotationService = {
    // Generates a NEW Recovery Phrase, rotates the Recovery Key Wrapper, keeps Password Wrapper same (or rotates it too if we had the password).
    // Actually, to rotate the Emergency Kit, we just need to generate a new Mnemonic, and re-wrap the SAME Master Key with it.
    // The Master Key itself DOES NOT CHANGE (to avoid re-encrypting all data).
    // only the "Door" (Key Wrapper) changes.
    async rotateRecoveryKey(_masterKeyBase64: string): Promise<{ mnemonic: string, keyWrappedRk: string, recoveryHash: string }> {
        // const mnemonic = MasterKeyService.generateRecoveryPhrase();
        // const salt = MasterKeyService.generateSalt(); 

        throw new Error("Salt must be provided from current user record. Use rotateRecoveryKeyWithSalt instead.");
    },

    async rotateRecoveryKeyWithSalt(masterKeyBase64: string, salt: string): Promise<{ mnemonic: string, keyWrappedRk: string, recoveryHash: string }> {
        const mnemonic = MasterKeyService.generateRecoveryPhrase();
        const keyWrappedRk = await MasterKeyService.wrapKey(masterKeyBase64, mnemonic, salt);
        const recoveryHash = await MasterKeyService.hashString(mnemonic);

        return { mnemonic, keyWrappedRk, recoveryHash };
    }
};
