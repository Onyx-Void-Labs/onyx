import React, { createContext, useContext, useEffect, useState } from 'react';
import PocketBase, { AuthModel } from 'pocketbase';

// Context
interface PocketBaseContextType {
    pb: PocketBase;
    user: AuthModel | null;
    isConnected: boolean;
    login: (email: string, password: string) => Promise<void>;
    logout: () => void;
}

const PocketBaseContext = createContext<PocketBaseContextType | null>(null);

// Constants
// DEV MODE: Localhost
// PROD MODE: Your RackNerd VPS IP
const PB_URL = 'http://127.0.0.1:8090';

import { usePocketBaseSync } from '../hooks/usePocketBaseSync';

// ... (existing imports)

export function PocketBaseProvider({ children }: { children: React.ReactNode }) {
    const [pb] = useState(() => new PocketBase(PB_URL));
    const [user, setUser] = useState<AuthModel | null>(pb.authStore.model);
    const [isConnected, setIsConnected] = useState(false);

    // We need a way to trigger refreshNotes globally or locally? 
    // Ideally the hook handles the refresh by calling a passed function.
    // The previous implementation of Sidebar/App owned the notes list.
    // How do we refresh the App's notes list from here?
    // We might need to expose a `refresh` method in the Context or use an Event Bus.
    // For now, let's assume usePocketBaseSync will just do its thing in background
    // BUT it needs to update the UI.
    // `usePocketBaseSync` takes `refreshNotes` as arg.
    // We don't have access to `setNotes` here.

    // Quick Fix: Dispatch Custom Event 'onyx:refresh-notes'
    const notifyRefresh = () => {
        window.dispatchEvent(new CustomEvent('onyx:refresh-notes'));
    };

    usePocketBaseSync(pb, isConnected, user, notifyRefresh);

    useEffect(() => {
        // ... (existing connection check)
        // Sync Auth State
        const unsubscribe = pb.authStore.onChange((_token, model) => {
            setUser(model);
        });

        // Check Connection (Heartbeat)
        const checkConnection = async () => {
            try {
                // Lightweight health check
                await pb.health.check();
                setIsConnected(true);
            } catch (e) {
                // console.warn("PocketBase disconnected", e); // Silence logs in dev
                setIsConnected(false);
            }
        };

        checkConnection();
        const interval = setInterval(checkConnection, 5000); // Check every 5s

        return () => {
            unsubscribe();
            clearInterval(interval);
        };
    }, [pb]);

    const login = async (email: string, password: string) => {
        try {
            // 1. Try User Login
            console.log("Attempting User Login...");
            await pb.collection('users').authWithPassword(email, password);
            console.log("User Login Success");
        } catch (userErr: any) {
            console.warn("User login failed:", userErr.message);

            // 2. Try Admin Login (Fallback)
            try {
                console.log("Attempting Admin Login...");
                await pb.admins.authWithPassword(email, password);
                console.log("Admin Login Success");
            } catch (adminErr: any) {
                console.error("Admin login also failed:", adminErr);
                // Throw a clear error message
                throw new Error("Authentication failed. Please check your credentials.");
            }
        }
    };

    const logout = () => {
        pb.authStore.clear();
    };

    return (
        <PocketBaseContext.Provider value={{ pb, user, isConnected, login, logout }}>
            {children}
        </PocketBaseContext.Provider>
    );
}

export const usePocketBase = () => {
    const ctx = useContext(PocketBaseContext);
    if (!ctx) throw new Error("usePocketBase must be used within a PocketBaseProvider");
    return ctx;
};
