routerAdd("POST", "/api/zk-reset-password", (c) => {
    const data = $apis.requestInfo(c).data;
    const emailHash = data.emailHash;
    const recoveryHash = data.recoveryHash; // Client sends SHA256(mnemonic)
    const newPassword = data.newPassword;

    if (!emailHash || !recoveryHash || !newPassword) {
        throw new BadRequestError("Missing required fields.");
    }

    // specific query to find user by email (username is also email hash usually in this app?)
    // In AuthModal, we create authWithPassword(pseudonym, ...).
    // So email in PB is the pseudonym.

    // Find user by email (pseudonym) and recovery_hash
    // We cannot trust client-provided ID, we must look up by unique email.

    const result = $app.dao().findAuthRecordByEmail("users", emailHash);
    if (!result) {
        throw new BadRequestError("User not found.");
    }

    const storedHash = result.get("recovery_hash");

    // Constant time comparison roughly, but strictly just string check here
    if (storedHash !== recoveryHash) {
        throw new BadRequestError("Invalid Recovery Phrase.");
    }

    // Verification successful. Reset password.
    result.setPassword(newPassword);
    $app.dao().saveRecord(result);

    return c.json(200, {
        success: true,
        enc_salt: result.get("enc_salt"),
        key_wrapped_rk: result.get("key_wrapped_rk")
    });
});
