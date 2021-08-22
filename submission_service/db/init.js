db.createCollection("contests")
db.contests.createIndex(
    { "users.username": 1 },
    {
        "name": "authUserIndex",
        "unique": true,
        "sparse": true
    }
)

db.contests.insertOne({
    "name": "default_contest",
    "description": "Default contest",
    "startTime": ISODate(),
    "endTime": ISODate(),
    "problems": [],
    "users": [],
    "announcements": [],
    "questions": []
})