import { Server } from '@hocuspocus/server'
import { Database } from '@hocuspocus/extension-database'
import PocketBase from 'pocketbase'
import DatabaseDriver from 'better-sqlite3'
import express from 'express'
import expressWebsockets from 'express-ws'
import cors from 'cors'
import Stripe from 'stripe'
import dotenv from 'dotenv'

dotenv.config()

// --- CONFIGURATION ---
const PORT = process.env.PORT || 1234
const PB_URL = process.env.POCKETBASE_URL || 'http://pocketbase:8090'
const STRIPE_SECRET_KEY = process.env.STRIPE_SECRET_KEY
const stripe = new Stripe(STRIPE_SECRET_KEY)

if (!STRIPE_SECRET_KEY) {
    console.warn("⚠️  STRIPE_SECRET_KEY is missing! Payments will fail.")
}

// --- DATABASE SETUP ---
import fs from 'fs';
if (!fs.existsSync('./data')) {
    fs.mkdirSync('./data');
}
const db = new DatabaseDriver('./data/hocuspocus.db')

db.exec(`
  CREATE TABLE IF NOT EXISTS documents (
    name TEXT PRIMARY KEY,
    data BLOB
  )
`)

const getDoc = db.prepare('SELECT data FROM documents WHERE name = ?')
const upsertDoc = db.prepare(`
  INSERT INTO documents (name, data) 
  VALUES (?, ?) 
  ON CONFLICT(name) DO UPDATE SET data = excluded.data
`)

// --- SERVER SETUP ---
const app = express()
const { app: wsApp } = expressWebsockets(app)

app.use(cors())
// Raw body for webhook, JSON for API
app.use('/api/webhook', express.raw({ type: 'application/json' }))
app.use(express.json())

// --- HOCUSPOCUS SETUP ---
const hocuspocus = Server.configure({
    async onAuthenticate(data) {
        const { token } = data
        if (!token) throw new Error('No token provided')

        try {
            const pb = new PocketBase(PB_URL)
            pb.authStore.save(token, null)
            const authData = await pb.collection('users').authRefresh()

            return {
                user: {
                    id: authData.record.id,
                    email: authData.record.email
                }
            }
        } catch (err) {
            console.error("Auth Failed:", err.message)
            throw new Error('Unauthorized')
        }
    },

    extensions: [
        new Database({
            debounce: 100,
            fetch: async ({ documentName }) => {
                const row = getDoc.get(documentName)
                return row ? row.data : null
            },
            store: async ({ documentName, state }) => {
                upsertDoc.run(documentName, state)
            },
        }),
    ],
})

// Bind Hocuspocus to WebSocket endpoint
wsApp.ws('/', (websocket, request) => {
    hocuspocus.handleConnection(websocket, request)
})

// --- STRIPE API ---

const PLANS = {
    'price_1SyQmYE5jY0cQOyplB1qfQfF': { bytes: 25 * 1024 * 1024 * 1024 },   // 25GB
    'price_1SyQnPE5jY0cQOyprIh1rqSF': { bytes: 50 * 1024 * 1024 * 1024 },     // 50GB
    'price_1SyQnwE5jY0cQOypnTYdofMr': { bytes: 200 * 1024 * 1024 * 1024 }   // 200GB
}

// Create Subscription Endpoint
app.post('/api/create-subscription', async (req, res) => {
    try {
        const { email, priceId, userId, promoCode } = req.body;

        // 0. Lookup Promo Code (if provided)
        let promotionCodeId = undefined;
        if (promoCode) {
            const promos = await stripe.promotionCodes.list({
                code: promoCode,
                active: true,
                limit: 1
            });
            if (promos.data.length > 0) {
                promotionCodeId = promos.data[0].id;
            } else {
                return res.status(400).json({ error: { message: "Invalid or expired promo code." } });
            }
        }

        // 1. Find or Create Customer
        let customers = await stripe.customers.list({ email: email, limit: 1 });
        let customer = customers.data.length > 0 ? customers.data[0] : null;

        if (!customer) {
            customer = await stripe.customers.create({
                email: email,
                metadata: { userId: userId }
            });
        }

        // 2. Create Subscription
        const subscription = await stripe.subscriptions.create({
            customer: customer.id,
            items: [{ price: priceId }],
            promotion_code: promotionCodeId, // Apply discount if valid
            automatic_tax: { enabled: true },
            payment_behavior: 'default_incomplete',
            payment_settings: { save_default_payment_method: 'on_subscription' },
            expand: ['latest_invoice.payment_intent'],
            metadata: { userId: userId }
        });

        const invoice = subscription.latest_invoice;

        res.json({
            subscriptionId: subscription.id,
            clientSecret: invoice.payment_intent.client_secret,
            amount: invoice.amount_due,
            currency: invoice.currency,
            subtotal: invoice.subtotal || 0,
            tax: invoice.tax || 0,
            discount: invoice.total_discount_amounts?.[0]?.amount || 0
        });

    } catch (error) {
        console.error("Stripe Error:", error);
        res.status(400).json({ error: { message: error.message } });
    }
});

// Webhook Handler
app.post('/api/webhook', async (req, res) => {
    const sig = req.headers['stripe-signature'];
    const webhookSecret = process.env.STRIPE_WEBHOOK_SECRET;

    let event;

    try {
        if (webhookSecret) {
            event = stripe.webhooks.constructEvent(req.body, sig, webhookSecret);
        } else {
            event = JSON.parse(req.body);
        }
    } catch (err) {
        console.error(`Webhook Error: ${err.message}`);
        return res.status(400).send(`Webhook Error: ${err.message}`);
    }

    // Handle Events
    switch (event.type) {
        case 'invoice.payment_succeeded':
            const invoice = event.data.object;
            const userId = invoice.subscription_details?.metadata?.userId; // Custom metadata
            // TODO: Update PocketBase user storage limit here
            console.log(`Payment succeeded for user ${userId || 'unknown'}`);
            break;
        default:
            console.log(`Unhandled event type ${event.type}`);
    }

    res.json({ received: true });
});


// Start Server
app.listen(PORT, '0.0.0.0', () => {
    console.log(`[Onyx API] Running on port ${PORT}`);
    console.log(`[Onyx Sync] WebSockets ready`);
})
