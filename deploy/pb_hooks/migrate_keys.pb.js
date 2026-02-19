onAfterBootstrap((e) => {
    try {
        const collection = $app.dao().findCollectionByNameOrId("users");
        let changed = false;

        const fieldsToAdd = [
            { name: "enc_salt", type: "text" },
            { name: "key_wrapped_pw", type: "text" },
            { name: "key_wrapped_rk", type: "text" },
            { name: "recovery_hash", type: "text" }
        ];

        fieldsToAdd.forEach(f => {
            if (!collection.schema.getFieldByName(f.name)) {
                console.log(`Adding '${f.name}' field to users collection...`);
                // @ts-ignore
                const field = new SchemaField({
                    "system": false,
                    "id": "",
                    "name": f.name,
                    "type": f.type,
                    "required": false,
                    "unique": false,
                    "options": { "pattern": "" }
                });
                collection.schema.addField(field);
                changed = true;
            }
        });

        if (changed) {
            $app.dao().saveCollection(collection);
            console.log("Successfully added Master Key fields.");
        }
    } catch (err) {
        console.error("Migration error (Keys):", err);
    }
})
