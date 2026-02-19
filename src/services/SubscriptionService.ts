import { pb } from '../lib/pocketbase';

const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:1234'; // Points to Hocuspocus/Express

export interface CreateSubscriptionResponse {
    subscriptionId: string;
    clientSecret: string;
    amount: number;
    currency: string;
    subtotal: number;
    tax: number;
    discount: number;
}

export const SubscriptionService = {
    async createSubscription(priceId: string, promoCode?: string) {
        const user = pb.authStore.model;
        if (!user) throw new Error("Not authenticated");

        const response = await fetch(`${API_URL}/api/create-subscription`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                email: user.email,
                userId: user.id,
                priceId,
                promoCode // Optional
            })
        });

        if (!response.ok) {
            const errorData = await response.json();
            throw new Error(errorData.error?.message || "Failed to create subscription");
        }

        return await response.json();
    }
};
