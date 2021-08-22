db.createCollection("submissions", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["authorUsername", "problemId", "created", "source", "state"],
      _id: { bsonType: "string" }, // submission id
      properties: {
        authorUsername: { bsonType: "string" },
        problemId: { bsonType: "int" },
        created: { bsonType: "timestamp" },
        source: { bsonType: "binData" },
        state: { enum: ['Pending','Evaluated'] },
        compilation: {
          bsonType: "object",
          required: ["outcome", "timeMs", "memoryB"],
          properties: {
            outcome: { bsonType: "string" },
            timeMs: { bsonType: "int" },
            memoryB: { bsonType: "int" },
            error: { bsonType: "string" }
          }
        },
        evaluation: {
          bsonType: "object",
          required: ["subtasks"],
          properties: {
            subtasks: {
              bsonType: "array",
              items: {
                bsonType: "object",
                required: ["testcases", "subtaskScore"],
                properties: {
                  subtaskScore: { 
                    oneOf: [ 
                      { bsonType: "bool"},
                      { bsonType: "double"}
                    ]
                  },
                  testcases: {
                    bsonType: "array",
                    items: {
                      bsonType: "object",
                      required: ["testcaseOutcome", "timeMs", "memoryB"],
                      properties: {
                        testcaseOutcome: {
                          bsonType: "object",
                          required: ["verdict"],
                          properties: {
                              verdict: { bsonType : "string" } ,
                              score: { bsonType: "double" },
                          }
                        },
                        timeMs: { bsonType: "int" },
                        memoryB: { bsonType: "int" },
                      }
                    }
                  }
                }
              }
            }
          }
        },
        overallScore: { 
          oneOf: [ 
            { bsonType: "bool"},
            { bsonType: "double"}
          ]
        }
      }
    }
  }
})

// create index for field authorUsername
db.collection.createIndex( { authorUsername: "text" } )