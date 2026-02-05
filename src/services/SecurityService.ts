
// SecurityService.ts
// Handles all Client-Side Encryption operations using Web Crypto API.

const ALGORITHM = 'AES-GCM';
const KEY_DERIVATION = 'PBKDF2';
const HASH = 'SHA-256';
const ITERATIONS = 100000; // Strong standard
const SALT_LENGTH = 16;
const IV_LENGTH = 12; // Standard for GCM

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

// 1. Derive Key from Password
async function deriveKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
    const textEncoder = new TextEncoder();
    const passwordBuffer = textEncoder.encode(password);

    const importedKey = await window.crypto.subtle.importKey(
        'raw',
        passwordBuffer,
        KEY_DERIVATION,
        false,
        ['deriveKey']
    );

    return window.crypto.subtle.deriveKey(
        {
            name: KEY_DERIVATION,
            salt: salt as any, // Cast to any to bypass SharedArrayBuffer strictness lint
            iterations: ITERATIONS,
            hash: HASH,
        },
        importedKey,
        { name: ALGORITHM, length: 256 },
        false,
        ['encrypt', 'decrypt']
    );
}

// 2. Encrypt Text
export async function encryptNote(content: string, password: string): Promise<EncryptedNote> {
    const textEncoder = new TextEncoder();
    const encodedContent = textEncoder.encode(content);

    // Generate random salts
    const salt = window.crypto.getRandomValues(new Uint8Array(SALT_LENGTH));
    const iv = window.crypto.getRandomValues(new Uint8Array(IV_LENGTH));

    const key = await deriveKey(password, salt);

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
        salt: arrayBufferToBase64(salt.buffer),
        data: arrayBufferToBase64(ciphertext),
    };
}

// 3. Decrypt Text
export async function decryptNote(encrypted: EncryptedNote, password: string): Promise<string> {
    try {
        const salt = new Uint8Array(base64ToArrayBuffer(encrypted.salt));
        const iv = new Uint8Array(base64ToArrayBuffer(encrypted.iv));
        const data = base64ToArrayBuffer(encrypted.data);

        const key = await deriveKey(password, salt);

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
        throw new Error("Invalid password or corrupted data");
    }
}
