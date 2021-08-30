db.createCollection("submissions", {
  validator: {
    $jsonSchema: {
      bsonType: "object",
      required: ["_id", "user", "problemId", "created", "source", "programmingLanguage", "state"],
      properties: {
        _id: { bsonType: "int" }, // submission id
        user: { bsonType: "string" },
        problemId: { bsonType: "int" },
        created: { bsonType: "timestamp" },
        source: { bsonType: "binData" },
        programmingLanguage: { 
          bsonType: "string",
          enum: ["RUST", "CPP"] // ...
        },
        state: { 
          bsonType: "string",
          enum: ["PENDING","EVALUATED","ABORTED"]
        },
        compilation: {
          bsonType: "object",
          required: ["outcome", "timeNs", "memoryB"],
          properties: {
            outcome: { 
              bsonType: "string",
              enum: ["NONE", "SUCCESS", "REJECTED", "TLE", "MLE", "RTE"]
            },
            timeNs: { bsonType: "int" },
            memoryB: { bsonType: "int" },
            error: { bsonType: "string" }
          }
        }, // EvaluationResult.compilation_result
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
                  }, // SubtaskResult.subtask_score
                  testcases: {
                    bsonType: "array",
                    items: {
                      bsonType: "object",
                      required: ["outcome", "score", "timeNs", "memoryB"], 
                      properties: {
                        outcome: {
                          bsonType: "string",
                          enum: ["NONE", "OK", "TLE", "MLE", "CHECKER_ERROR"]
                        }, // TestcaseResult.outcome
                        score: { 
                          oneOf: [ 
                            { bsonType: "bool"},
                            { bsonType: "double"}
                          ]
                        }, //TestcaseResult.score
                        timeNs: { bsonType: "int" }, // TestcaseResult.used_resources
                        memoryB: { bsonType: "int" }, // TestcaseResult.used_resources
                      }
                    }
                  } // SubtaskResult.testcase_results
                }
              }
            }
          } // EvaluationResult.subtask_results
        },
        overallScore: { 
          oneOf: [ 
            { bsonType: "bool"},
            { bsonType: "double"}
          ]
        } // EvaluationResult.overall_score
      }
    }
  }
})

// create index for field user
db.collection.createIndex( { user: "text" } )