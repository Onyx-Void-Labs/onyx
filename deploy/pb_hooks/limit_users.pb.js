/// <reference path="../pb_data/types.d.ts" />

onRecordBeforeCreateRequest((e) => {
    const hwid = e.record.get("hwid")

    // If no HWID is provided, we might want to block or allow (for legacy/testing).
    // For abuse prevention, we should require it.
    if (!hwid) {
        throw new BadRequestError("Device ID (hwid) is required.")
    }

    // Check how many users have this hwid
    const result = $app.dao().findRecordsByFilter("users", `hwid = '${hwid}'`)

    // Limit to 2 accounts
    if (result.length >= 2) {
        throw new BadRequestError("Maximum account limit reached for this device.")
    }

    // --- Captcha Verification ---
    const token = e.httpContext.request().header.get("X-Captcha-Token")
    if (!token) {
        throw new BadRequestError("Captcha token missing.")
    }

    try {
        // Verify with Cloudflare
        const secret = $os.getenv("TURNSTILE_SECRET_KEY");
        if (!secret) {
            // Fallback for dev/testing if env not set, OR throw error in prod
            // For now, let's throw to ensure they configure it.
            throw new Error("Backend misconfiguration: TURNSTILE_SECRET_KEY not set.");
        }

        const res = $http.send({
            url: "https://challenges.cloudflare.com/turnstile/v0/siteverify",
            method: "POST",
            body: JSON.stringify({
                secret: secret,
                response: token
            }),
            headers: { "Content-Type": "application/json" }
        })

        if (res.statusCode !== 200 || !res.json.success) {
            throw new BadRequestError("Captcha verification failed.")
        }
    } catch (err) {
        // If network fail, maybe allow or block? Block is safer.
        throw new BadRequestError("Captcha check failed: " + err.message)
    }

}, "users")
