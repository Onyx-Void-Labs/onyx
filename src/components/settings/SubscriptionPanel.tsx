import React, { useState } from 'react';
import { loadStripe } from '@stripe/stripe-js';
import { Elements, PaymentElement, useStripe, useElements } from '@stripe/react-stripe-js';
import { SubscriptionService } from '../../services/SubscriptionService';
import { Loader2, Check, Shield } from 'lucide-react';

// Initialize Stripe
const stripePromise = loadStripe(import.meta.env.VITE_STRIPE_PUBLISHABLE_KEY);

const PLANS = [
    {
        id: 'price_1SyQmYE5jY0cQOyplB1qfQfF', // Basic Plan
        name: 'Basic',
        price: '$1',
        period: '/mo',
        storage: '25 GB',
        features: ['Encrypted Sync', 'Mobile Access'],
        color: 'from-zinc-500 to-zinc-700'
    },
    {
        id: 'price_1SyQnPE5jY0cQOyprIh1rqSF', // Pro Plan
        name: 'Pro',
        price: '$2',
        period: '/mo',
        storage: '50 GB',
        features: ['Priority Support', 'Version History', 'Everything in Basic'],
        popular: true,
        color: 'from-purple-600 to-indigo-600'
    },
    {
        id: 'price_1SyQnwE5jY0cQOypnTYdofMr', // Ultra Plan
        name: 'Ultra',
        price: '$3',
        period: '/mo',
        storage: '200 GB',
        features: ['AI Assistant Unlocked', 'Team Sharing', 'Everything in Pro'],
        color: 'from-emerald-500 to-teal-600'
    }
];

const CheckoutForm = ({ onSuccess, amount, currency, subtotal, tax, discount }: {
    onSuccess: () => void,
    amount: number | null,
    currency: string | null,
    subtotal: number | null,
    tax: number | null,
    discount: number | null
}) => {
    const stripe = useStripe();
    const elements = useElements();
    const [message, setMessage] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(false);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();

        if (!stripe || !elements) return;

        setIsLoading(true);

        const { error } = await stripe.confirmPayment({
            elements,
            confirmParams: {
                // Return URL is required but we handle it in-app mostly.
                // For a native/electron app, we might need a special handler or just let it redirect.
                return_url: window.location.origin,
            },
            redirect: "if_required" // Important: Avoid redirecting if possible (for In-App feel)
        });

        if (error) {
            setMessage(error.message || "An unexpected error occurred.");
        } else {
            setMessage("Payment Successful!");
            onSuccess();
        }

        setIsLoading(false);
    };


    const formatPrice = (amount: number | null | undefined, currency = 'usd') => {
        const val = amount || 0;
        return new Intl.NumberFormat('en-US', { style: 'currency', currency }).format(val / 100);
    };

    return (
        <form onSubmit={handleSubmit} className="space-y-6 animate-in slide-in-from-bottom-4 duration-500">
            {/* Order Summary */}
            <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-4 space-y-2 text-sm text-zinc-300">
                <div className="flex justify-between">
                    <span>Subtotal</span>
                    <span>{formatPrice(subtotal, currency || 'usd')}</span>
                </div>
                {discount && discount > 0 ? (
                    <div className="flex justify-between text-emerald-400">
                        <span>Discount</span>
                        <span>-{formatPrice(discount, currency || 'usd')}</span>
                    </div>
                ) : null}
                <div className="flex justify-between text-zinc-500">
                    <span>Tax</span>
                    <span>{formatPrice(tax, currency || 'usd')}</span>
                </div>
                <div className="border-t border-zinc-800 pt-2 flex justify-between font-bold text-white text-base">
                    <span>Total</span>
                    <span>{formatPrice(amount, currency || 'usd')}</span>
                </div>
            </div>

            <div className="p-4 bg-zinc-900/50 border border-zinc-800 rounded-xl">
                {/* Wallets: Google/Apple Pay need 'requestPayerName' sometimes for full functionality, but 'auto' is usually enough */}
                <PaymentElement id="payment-element" options={{ layout: 'tabs', wallets: { applePay: 'auto', googlePay: 'auto' } }} />
            </div>

            {message && (
                <div className={`text-xs p-3 rounded-lg ${message.includes("Success") ? "bg-green-500/10 text-green-400" : "bg-red-500/10 text-red-400"}`}>
                    {message}
                </div>
            )}

            <button disabled={isLoading || !stripe || !elements} id="submit" className="w-full py-3 bg-white text-black font-black uppercase tracking-widest rounded-xl hover:bg-zinc-200 transition-all shadow-xl shadow-white/5 disabled:opacity-50 flex justify-center gap-2">
                {isLoading ? <Loader2 className="animate-spin" size={16} /> : `Pay ${amount && currency ? formatPrice(amount, currency) : 'Now'}`}
            </button>
        </form>
    );
};

export default function SubscriptionPanel() {
    const [selectedPlan, setSelectedPlan] = useState<string | null>(null);
    const [clientSecret, setClientSecret] = useState<string | null>(null);
    const [amount, setAmount] = useState<number | null>(null);
    const [currency, setCurrency] = useState<string | null>(null);
    const [subtotal, setSubtotal] = useState<number | null>(null);
    const [tax, setTax] = useState<number | null>(null);
    const [discount, setDiscount] = useState<number | null>(null);
    const [promoCode, setPromoCode] = useState(""); // User input
    const [appliedPromo, setAppliedPromo] = useState<string | null>(null); // For display after success
    const [loadingSecret, setLoadingSecret] = useState(false);
    const [success, setSuccess] = useState(false);

    const handleSelectPlan = async (priceId: string) => {
        setSelectedPlan(priceId);
        setLoadingSecret(true);
        try {
            const data = await SubscriptionService.createSubscription(priceId, promoCode);
            setClientSecret(data.clientSecret);
            setAmount(data.amount);
            setCurrency(data.currency);
            setSubtotal(data.subtotal);
            setTax(data.tax);
            setDiscount(data.discount);
            if (promoCode) setAppliedPromo(promoCode);
        } catch (err: any) {
            console.error(err);
            alert("Failed to start subscription: " + err.message);
            setSelectedPlan(null);
        } finally {
            setLoadingSecret(false);
        }
    };

    if (success) {
        return (
            <div className="flex flex-col items-center justify-center p-12 text-center space-y-6 animate-in zoom-in-95 duration-500">
                <div className="w-24 h-24 bg-green-500/20 text-green-400 rounded-full flex items-center justify-center mb-4">
                    <Check size={48} />
                </div>
                <h2 className="text-3xl font-black text-white uppercase tracking-tighter">Upgrade Complete</h2>
                <p className="text-zinc-400">Your storage limit has been increased. Thank you for supporting Onyx.</p>
            </div>
        );
    }

    return (
        <div className="space-y-8">
            {!clientSecret ? (
                <>
                    <div className="text-center space-y-2 mb-8">
                        <h2 className="text-2xl font-black text-white uppercase tracking-tight">Choose Your Plan</h2>
                        <p className="text-zinc-500 text-xs font-mono">SECURE PAYMENTS BY STRIPE • CANCEL ANYTIME</p>
                    </div>

                    {/* Promo Code Input - MOVED TO CHECKOUT VIEW (below) or kept here? 
                        User said: "discount should be in the next window when you choose cards" 
                        But we need the code BEFORE generating the secret to get the right price.
                        So we'll keep a "Pre-fill" here, but maybe allow editing in the next step? 
                        Simpler: Keep it here for now, but make it look like part of the plan selection.
                    */}

                    <div className="max-w-xs mx-auto mb-10">
                        <div className="relative group">
                            <div className="absolute -inset-0.5 bg-gradient-to-r from-purple-600 to-indigo-600 rounded-lg opacity-20 group-hover:opacity-40 transition duration-500 blur"></div>
                            <input
                                type="text"
                                placeholder="Have a Promo Code?"
                                value={promoCode}
                                onChange={(e) => setPromoCode(e.target.value)}
                                className="relative w-full bg-zinc-900 border border-zinc-700 text-zinc-100 text-xs font-mono py-2 px-4 rounded-lg focus:outline-none focus:border-purple-500 transition-all text-center uppercase tracking-widest placeholder:normal-case placeholder:tracking-normal"
                            />
                        </div>
                    </div>

                    <div className="grid grid-cols-3 gap-6">
                        {PLANS.map((plan) => (
                            <div key={plan.id} className={`relative p-6 bg-zinc-900/40 border ${plan.popular ? 'border-purple-500/50' : 'border-zinc-800'} rounded-3xl hover:bg-zinc-900 transition-all group overflow-hidden`}>
                                {plan.popular && (
                                    <div className="absolute top-0 right-0 px-3 py-1 bg-purple-600 text-[9px] font-black text-white uppercase tracking-widest rounded-bl-xl">Popular</div>
                                )}

                                <h3 className="text-lg font-bold text-white mb-2">{plan.name}</h3>
                                <div className="flex items-baseline gap-1 mb-4">
                                    <span className="text-3xl font-black text-white tracking-tighter">{plan.price}</span>
                                    <span className="text-xs text-zinc-500">{plan.period}</span>
                                </div>

                                <div className="text-xl font-black bg-clip-text text-transparent bg-gradient-to-r from-white to-zinc-500 mb-6">
                                    {plan.storage}
                                </div>

                                <ul className="space-y-3 mb-8">
                                    {plan.features.map((feat, i) => (
                                        <li key={i} className="flex items-center gap-2 text-[10px] text-zinc-400">
                                            <Check size={12} className="text-emerald-500" />
                                            {feat}
                                        </li>
                                    ))}
                                </ul>

                                <button
                                    onClick={() => handleSelectPlan(plan.id)}
                                    disabled={loadingSecret}
                                    className={`w-full py-3 rounded-xl text-[10px] font-black uppercase tracking-widest transition-all ${plan.popular
                                        ? 'bg-purple-600 hover:bg-purple-500 text-white shadow-lg shadow-purple-500/20'
                                        : 'bg-zinc-800 hover:bg-zinc-700 text-zinc-300'
                                        }`}
                                >
                                    {loadingSecret && selectedPlan === plan.id ? <Loader2 className="animate-spin mx-auto" size={14} /> : 'Select Plan'}
                                </button>
                            </div>
                        ))}
                    </div>
                </>
            ) : (
                <div className="max-w-md mx-auto">
                    <div className="mb-6 flex items-center justify-between">
                        <button onClick={() => setClientSecret(null)} className="text-xs text-zinc-500 hover:text-white transition-colors">← Back to Plans</button>
                        <div className="text-xs font-bold text-zinc-400 uppercase tracking-widest flex items-center gap-2">
                            <Shield size={12} className="text-emerald-500" />
                            Secure Checkout
                        </div>
                    </div>

                    {appliedPromo && (
                        <div className="mb-6 p-3 bg-emerald-500/10 border border-emerald-500/20 rounded-xl flex items-center justify-center gap-2 text-emerald-400 text-xs font-bold uppercase tracking-wide animate-in fade-in slide-in-from-top-2">
                            <Check size={12} /> Code Applied: {appliedPromo}
                        </div>
                    )}

                    <Elements options={{ clientSecret, appearance: { theme: 'night', variables: { colorPrimary: '#9333ea' } } }} stripe={stripePromise}>
                        <CheckoutForm
                            onSuccess={() => setSuccess(true)}
                            amount={amount}
                            currency={currency}
                            subtotal={subtotal}
                            tax={tax}
                            discount={discount}
                        />
                    </Elements>
                </div>
            )}
        </div>
    );
}
