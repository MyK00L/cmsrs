db.createCollection("contest_metadata", {
    max: 1, // caps the number of documents in this collection to 1
    validator: {
        $jsonSchema: {
            bsonType: "object",
            required: ["name", "description"],
            properties: {
                name: { bsonType: "string" },
                description: { bsonType: "string" },
                startTime: { bsonType: "timestamp" }, // missing means there is no start time
                endTime: { bsonType: "timestamp" }, // missing means there is no end time
            }
        }
    }
})

db.createCollection("problems", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            _id: { bsonType: "int" }, // problem id
            required: ["name", "longName", "statement"],
            properties: {
                name: { bsonType: "string" },
                longName: { bsonType: "string" },
                statement: { bsonType: "binData" }
                // testcasesPerSubtask: {
                //     bsonType: "array",
                //     items: {
                //         bsonType: "int"
                //     }
                // },
                // timeLimitNs: { bsonType: "int" }, add when needed
                // memoryLimitB: { bsonType: "int" }, add when needed
                // sourceSizeLimitB: { bsonType: "int" }, add when needed
                // score: { bsonType: "..." } add when needed
                // problemType: { bsonType: "string" } add when needed
            }
        }
    }
})

db.createCollection("users", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            _id: { bsonType: "string" }, // username
            required: ["fullname", "password"],
            properties: {
                fullname: { bsonType: "string" },
                password: { bsonType: "string" },
            }
        }
    }
})

db.createCollection("announcements", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            _id: { bsonType: "int" }, // announcement id
            required: ["subject", "text", "created"],
            properties: {
                subject: { bsonType: "string" },
                problemId: { bsonType: "int" },
                text: { bsonType: "string" },
                to: { bsonType: "string" },
                created: { bsonType: "timestamp" }
            }
        }
    }
})

db.createCollection("questions", {
    validator: {
        $jsonSchema: {
            bsonType: "object",
            _id: { bsonType: "int" }, // question id
            required: ["subject", "text", "created"],
            properties: {
                subject: { bsonType: "string" },
                problemId: { bsonType: "int" },
                text: { bsonType: "string" },
                from: { bsonType: "string" },
                created: { bsonType: "timestamp" }
            }
        }
    }
})
