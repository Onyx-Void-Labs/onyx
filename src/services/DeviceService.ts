import FingerprintJS from '@fingerprintjs/fingerprintjs';

export const DeviceService = {
    async getFingerprint(): Promise<string> {
        try {
            const fp = await FingerprintJS.load();
            const result = await fp.get();
            return result.visitorId;
        } catch (error) {
            console.error("Failed to generate fingerprint:", error);
            // Fallback to a random ID if fingerprinting fails (not ideal but prevents signup block)
            // In a strict environment, we might want to block or flag this.
            return "unknown-" + Math.random().toString(36).substring(2, 15);
        }
    }
};
