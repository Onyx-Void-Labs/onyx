onAfterBootstrap((e) => {
    try {
        const collection = $app.dao().findCollectionByNameOrId("users");

        // check if field exists, if not add it
        if (!collection.schema.getFieldByName("hwid")) {
            console.log("Adding 'hwid' field to users collection...");

            // In PB JS hooks, we can't easily import strict Go types for SchemaField,
            // but we can manipulate the schema array or use raw JSON if possible.
            // Alternatively, we can use the dao saveCollection which validates it.
            // Let's try to parse the schema, add the field, and save.

            // Wait, simply constructing the field object might be key.
            // The safest way in JS hooks often involves avoiding complex type instantiation if not globally available.
            // But 'SchemaField' is usually exposed.

            const field = new SchemaField({
                "system": false,
                "id": "",
                "name": "hwid",
                "type": "text",
                "required": false,
                "presentable": false,
                "unique": false,
                "options": {
                    "min": null,
                    "max": null,
                    "pattern": ""
                }
            });

            collection.schema.addField(field);
            $app.dao().saveCollection(collection);
            console.log("Successfully added 'hwid' field.");
        }
    } catch (err) {
        console.error("Migration error:", err);
    }
})
