use mongodb::{
    bson::{doc, Document},
    options::{CreateCollectionOptions, UpdateOptions, ValidationAction, ValidationLevel},
    Database,
};

pub async fn init_contest_service_db(db: Database) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: consider using this validator syntax (might be slightly nicer):
    // https://docs.mongodb.com/v5.0/core/schema-validation/#other-query-expressions
    db.create_collection(
        "contest_metadata",
        CreateCollectionOptions::builder()
            .validator(doc! {
                "$jsonSchema": {
                    "bsonType": "object",
                    "required": ["name", "description", "startTime", "endTime"],
                    "properties": {
                        "name": { "bsonType": "string" },
                        "description": { "bsonType": "string" },
                        "startTime": { "bsonType": ["null","timestamp"] }, // null means there is no start time
                        "endTime": { "bsonType": ["null","timestamp"] } // null means there is no end time
                    }
                }
            })
            .validation_action(ValidationAction::Error)
            .validation_level(ValidationLevel::Strict)
            /*.capped(true)
            .max(1)
            .size(2_u64.pow(20))*/
            // cap disabled because you Cannot change the size of a document in a capped collection
            // Size must be set if we want to used capped. We set it to an unreachable value (1 MB) so
            // it will never be reached by our singleton document (enforced by max(1)).
            .build(),
    )
    .await?;

    db.create_collection(
        "problems",
        CreateCollectionOptions::builder()
            .validator(doc! {
                "$jsonSchema": {
                    "bsonType": "object",
                    "required": ["_id", "name", "longName"/*, "statement"*/],
                    "properties": {
                        "_id": { "bsonType": "long" }, // problem id
                        "name": { "bsonType": "string" },
                        "longName": { "bsonType": "string" },
                        "statement": { "bsonType": "binData" }
                    }
                }
            })
            .validation_action(ValidationAction::Error)
            .validation_level(ValidationLevel::Strict)
            .build(),
    )
    .await?;

    db.create_collection(
        "users",
        CreateCollectionOptions::builder()
            .validator(doc! {
                "$jsonSchema": {
                    "bsonType": "object",
                    "required": ["_id", "fullname", "password"],
                    "properties": {
                        "_id": { "bsonType": "string" }, // username
                        "fullname": { "bsonType": "string" },
                        "password": { "bsonType": "string" }
                    }
                }
            })
            .validation_action(ValidationAction::Error)
            .validation_level(ValidationLevel::Strict)
            .build(),
    )
    .await?;

    db.create_collection(
        "announcements",
        CreateCollectionOptions::builder()
            .validator(doc! {
                "$jsonSchema": {
                    "bsonType": "object",
                    "required": ["_id", "subject", "problemId", "text", "to", "created"],
                    "properties": {
                        "_id": { "bsonType": "long" }, // announcement id
                        "subject": { "bsonType": "string" },
                        "problemId": { "bsonType": ["null","int"] },
                        "text": { "bsonType": ["null","string"] },
                        "to": { "bsonType": ["null","string"] },
                        "created": { "bsonType": "timestamp" }
                    }
                }
            })
            .validation_action(ValidationAction::Error)
            .validation_level(ValidationLevel::Strict)
            .build(),
    )
    .await?;

    db.create_collection(
        "questions",
        CreateCollectionOptions::builder()
            .validator(doc! {
                "$jsonSchema": {
                    "bsonType": "object",
                    "required": ["_id", "subject", "problemId", "text", "from", "created"],
                    "properties": {
                        "_id": { "bsonType": "long" }, // question id
                        "subject": { "bsonType": "string" },
                        "problemId": { "bsonType": ["null","int"] },
                        "text": { "bsonType": "string" },
                        "from": { "bsonType": ["null","string"] },
                        "created": { "bsonType": "timestamp" }
                    }
                }
            })
            .validation_action(ValidationAction::Error)
            .validation_level(ValidationLevel::Strict)
            .build(),
    )
    .await?;

    let metadata = crate::mappings::contest::ContestMetadata::default();
    db.collection::<Document>("contest_metadata")
        .update_one(
            doc! {},
            doc! { "$set": Document::from(metadata) },
            UpdateOptions::builder().upsert(true).build(),
        )
        .await?;

    Ok(())
}
