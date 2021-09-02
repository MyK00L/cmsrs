#![recursion_limit="256"]

use rand::{Rng};
use core::panic;
use std::time::SystemTime;

use futures::stream::StreamExt;

use ::utils::{gen_uuid, mongo::*};
use mongodb::{Client, Database, bson::{Binary, Bson, Document, bson, doc, spec::{BinarySubtype, ElementType}}, options::{ClientOptions, CreateCollectionOptions, FindOptions, ValidationAction, ValidationLevel}};
use protos::{*, self, common::Resources, evaluation::{CompilationResult, EvaluationResult, SubtaskResult, TestcaseResult, compilation_result, testcase_result}, service::{dispatcher::{EvaluateSubmissionResponse, MockDispatcher}}};
use protos::service::submission::*;
use protos::scoring::*;
use protos::service::submission::submission_server::*;
use protos::service::dispatcher::dispatcher_server::*;
use protos::utils::*;
use tonic::{Request, Response, Status, transport::*};

#[cfg(test)]
mod tests;

const DUMMY_MESSAGE: String = String::new();
// TODO: remove credentials to connect to db.
const CONNECTION_STRING: &str = "mongodb://root:example@submission_service_db:27017/";

fn internal_error<T>(e: T) -> Status
where
    T: std::fmt::Debug,
{
    Status::internal(format!("{:?}", e))
}

fn expected_field(field_name: &str) -> String {
    format!("This should not happen. In this context {} is a required field in db", field_name)
}

async fn init_contest_service_db(db: Database) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: consider using this validator syntax (might be slightly nicer):
    // https://docs.mongodb.com/v5.0/core/schema-validation/#other-query-expressions
    db.create_collection(
        "submissions",
        CreateCollectionOptions::builder()
            .validator(doc! {
                "$jsonSchema": {
                    "bsonType": "object",
                    "required": ["_id", "user", "problemId", "created", "source", "programmingLanguage", "state"],
                    "properties": {
                        "_id": { "bsonType": "long" }, // submission id
                        "user": { "bsonType": "string" },
                        "problemId": { "bsonType": "long" },
                        "created": { "bsonType": "timestamp" },
                        "source": { "bsonType": "binData" },
                        "programmingLanguage": { 
                            "bsonType": "int",
                            "enum": [0, 1, 2]
                            /*
                            0 => NONE
                            1 => RUST
                            2 => CPP
                            ...
                            */
                        },
                        "state": { 
                            "bsonType": "int",
                            "enum": [0, 1, 2]
                            /*
                            0 => PENDING
                            1 => EVALUATED
                            2 => ABORTED
                            */
                        },
                        "compilation": {
                            "bsonType": "object",
                            "required": ["outcome", "timeNs", "memoryB"],
                            "properties": {
                                "outcome": { 
                                    "bsonType": "int",
                                    "enum": [0, 1, 2, 3, 4, 5]
                                    /*
                                    0 => NONE
                                    1 => SUCCESS
                                    2 => REJECTED
                                    3 => TLE
                                    4 => MLE
                                    5 => RTE
                                    */
                                },
                                "timeNs": { "bsonType": "long" },
                                "memoryB": { "bsonType": "long" },
                                "error": { "bsonType": "string" }
                            }
                        }, // EvaluationResult.compilation_result
                        "evaluation": {
                            "bsonType": "object",
                            "required": ["subtasks"],
                            "properties": {
                                "subtasks": {
                                "bsonType": "array",
                                "items": {
                                    "bsonType": "object",
                                    "required": ["testcases", "subtaskScore"],
                                    "properties": {
                                    "subtaskScore": { 
                                        "oneOf": [ 
                                            { "bsonType": "bool"},
                                            { "bsonType": "double"}
                                        ]
                                    }, // SubtaskResult.subtask_score
                                    "testcases": {
                                        "bsonType": "array",
                                        "items": {
                                            "bsonType": "object",
                                            "required": ["outcome", "score", "timeNs", "memoryB"], 
                                            "properties": {
                                                "outcome": {
                                                    "bsonType": "int",
                                                    "enum": [0, 1, 2, 3, 4, 5]
                                                    /*
                                                    0 => NONE
                                                    1 => OK
                                                    2 => TLE
                                                    3 => MLE
                                                    4 => RTE
                                                    5 => CHECKER_ERROR
                                                    */
                                                }, // TestcaseResult.outcome
                                                "score": { 
                                                    "oneOf": [ 
                                                        { "bsonType": "bool"},
                                                        { "bsonType": "double"}
                                                    ]
                                                }, //TestcaseResult.score
                                                "timeNs": { "bsonType": "long" }, // TestcaseResult.used_resources
                                                "memoryB": { "bsonType": "long" }, // TestcaseResult.used_resources
                                        }
                                        }
                                    } // SubtaskResult.testcase_results
                                    }
                                }
                                }
                            } // EvaluationResult.subtask_results
                        },
                        "overallScore": { 
                            "oneOf": [ 
                                { "bsonType": "bool"},
                                { "bsonType": "double"}
                            ]
                        } // EvaluationResult.overall_score
                    }
                }
            })
            .validation_action(ValidationAction::Error)
            .validation_level(ValidationLevel::Strict)
            .build()
    )
    .await?;
    
    Ok(())
}

#[derive(Debug)]
pub struct SubmissionService {
    db_client: Client,
}

impl SubmissionService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let db_client = Client::with_options(ClientOptions::parse(CONNECTION_STRING).await?)?;
        init_contest_service_db(db_client.database("submissionsdb")).await?;
        Ok(Self {
            db_client: db_client,
        })
    }

    fn get_collection(&self) -> mongodb::Collection<Document> {
        self.db_client
            .database("submissionsdb")
            .collection::<Document>("submissions")
    }
}

fn score_option_bson_to_struct(opt_bson_score: Option<&Bson>, expected: bool, expect_message: String) -> OneOfScore {
    OneOfScore {
        score: 
            if let Some(bson_score) = opt_bson_score {
                match bson_score.element_type() {
                    ElementType::Double => 
                        Some(one_of_score::Score::DoubleScore(bson_score.as_f64().unwrap())),
                    ElementType::Boolean =>
                        Some(one_of_score::Score::BoolScore(bson_score.as_bool().unwrap())),
                    _ => panic! ("score cannot have this type")
                }
            } else {
                if expected {
                    panic!("{}", expect_message.as_str())
                } else {
                    None
                }
            }
    }
}

fn score_struct_to_bson(score_struct: OneOfScore) -> Option<Bson> {
    match score_struct.score {
        Some(one_of_score::Score::DoubleScore(double_score)) => Some(Bson::Double(double_score)), 
        Some(one_of_score::Score::BoolScore(bool_score)) => Some(Bson::Boolean(bool_score)), 
        _ => None
    }
}

fn get_item_from_doc(doc: Document) -> get_submission_list_response::Item {
    get_submission_list_response::Item{
        submission_id: doc.get_i64("_id").unwrap() as u64,
        user: doc.get_str("user").unwrap().to_string(),
        problem_id: doc.get_i64("problemId").unwrap() as u64,
        timestamp: timestamp_to_systime(doc.get_timestamp("created").unwrap()).into(),
        state: doc.get_i32("state").expect(expected_field("state").as_str()),
        score: score_option_bson_to_struct(doc.get("overallScore"), false, DUMMY_MESSAGE)
    }
}

fn convert_to_i64(x: u64) -> i64 {
    x as i64
}

fn create_pending_submission_document(submission: evaluation::Submission) -> Document {
    doc! {
        "_id": convert_to_i64(gen_uuid()),
        "user": submission.user,
        "problemId": submission.problem_id as i64,
        "created": systime_to_timestamp(SystemTime::now()),
        "source": Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes: submission.source.code
        }),
        "state": SubmissionState::Pending as i32,
        "programmingLanguage": submission.source.lang
    }
}

fn duration_to_time_ns(duration: common::Duration) -> i64 {
    if duration.secs > (i32::MAX as u64) {
        (i32::MAX as i64) * 1_000_000_000 + (duration.nanos as i64)
    } else {
        (duration.secs as i64)* 1_000_000_000 + (duration.nanos as i64)
    }
}

fn time_ns_to_duration(time_ns: i64) -> common::Duration {
    common::Duration { 
        secs: (time_ns / 1_000_000_000) as u64, 
        nanos: (time_ns % 1_000_000_000) as u32
    }
}

fn compilation_data_to_db_obj(compilation_result: CompilationResult) -> Bson {
    let mut db_obj_document = doc! {
        "outcome": compilation_result.outcome,
        "timeNs": duration_to_time_ns(compilation_result.used_resources.time),
        "memoryB": convert_to_i64(compilation_result.used_resources.memory_bytes),
    };
    if let Some(err_msg) = compilation_result.error_message {
        db_obj_document.insert("error", err_msg);
    }
    db_obj_document.into()
}

fn testcase_data_to_db_obj(testcase_data: &TestcaseResult) -> Bson {
    bson! ({
        "outcome": testcase_data.outcome,
        "score": score_struct_to_bson(testcase_data.score.clone()).unwrap(),
        "timeNs": duration_to_time_ns(testcase_data.used_resources.time.clone()),
        "memoryB": convert_to_i64(testcase_data.used_resources.memory_bytes)
    })
}

fn subtask_data_to_db_obj(subtask_data: &SubtaskResult) -> Bson {
    bson! ({
        "subtaskScore": score_struct_to_bson(subtask_data.score.clone()),// TODO instead of this, we should invoke the lib function to calculate the submission score
        "testcases": 
            subtask_data.testcase_results
                .iter()
                .map(|testcase_data| testcase_data_to_db_obj(testcase_data))
                .collect::<Bson>()
    })
}

fn insert_evaluation_data_into_document(
    doc_updated: &mut Document, 
    evaluation_result: &EvaluationResult) {
        doc_updated.insert("compilation", compilation_data_to_db_obj(evaluation_result.compilation_result.clone()));
        if evaluation_result.compilation_result.outcome == compilation_result::Outcome::Success as i32 {
            // if compilation succeeded, then fill evaluation and score fields
            doc_updated.insert("evaluation", 
                bson! ({
                    "subtasks": 
                        evaluation_result.subtask_results 
                            .clone()
                            .iter()
                            .map(|subtask_data| {
                                subtask_data_to_db_obj(subtask_data)
                            })
                            .collect::<Bson>()
                })
            );

            // TODO instead of this, we should invoke the lib function to calculate the submission score
            doc_updated.insert(
                "overallScore", 
                score_struct_to_bson(evaluation_result.score.clone())
                    .expect("This should not happen. Dispatcher should guarantee 
                        that score exists if compilation succeeded."));
        } else {
            doc_updated.insert("overallScore", Bson::Double(0f64));
        }
}

fn compilation_doc_to_struct(compilation_doc: &Document) -> CompilationResult {
    CompilationResult {
        outcome: compilation_doc.get_i32("outcome").expect(expected_field("compilation").as_str()),
        used_resources: Resources {
            time: time_ns_to_duration(compilation_doc.get_i64("timeNs").expect(expected_field("timeNs").as_str())),
            memory_bytes: compilation_doc.get_i64("memoryB").expect(expected_field("memoryB").as_str()) as u64,
        },
        error_message: 
            match compilation_doc.get("error") {
                Some(bson_string) => Some(
                    bson_string.as_str()
                        .expect("This should not happen. \'error\' must be stored as Bson::String")
                        .to_string()
                        .clone() // necessary?
                    ),
                None => None
            }
    }
}

fn single_testcase_db_to_struct(testcase_doc: &Document) -> TestcaseResult {
    TestcaseResult {
        outcome:
            testcase_doc.get_i32("outcome").expect(expected_field("outcome").as_str()),
        used_resources: Resources {
            time: time_ns_to_duration(testcase_doc.get_i64("timeNs").expect(expected_field("timeNs").as_str())),
            memory_bytes: testcase_doc.get_i64("memoryB").expect(expected_field("memoryB").as_str()) as u64,
        },
        score:
            score_option_bson_to_struct(testcase_doc.get("score"), true, expected_field("score")) // expected
    }
}

fn single_subtask_db_to_struct(subtask_doc: &Document) -> SubtaskResult {
    let subtask_score_bson = subtask_doc.get("subtaskScore")
        .expect(expected_field("subtaskScore").as_str());
    
    SubtaskResult {
        testcase_results: subtask_doc.get_array("testcases")
            .expect(expected_field("testcases").as_str())
            .into_iter()
            .map(|bson_testcase| {
                let testcase = bson_testcase.as_document()
                    .unwrap(); 
                single_testcase_db_to_struct(testcase)
            })
            .collect::<Vec<TestcaseResult>>(),
        score: 
            score_option_bson_to_struct(Some(subtask_score_bson), false, DUMMY_MESSAGE)
    }
}

fn subtasks_db_to_struct(evaluation_doc: &Document) -> Vec<SubtaskResult> {
    evaluation_doc.get_array("subtasks")
        .expect(expected_field("subtasks").as_str())
        .into_iter()
        .map(|bson_subtask| {
            let subtask = bson_subtask.as_document()
                .unwrap(); 
            single_subtask_db_to_struct(subtask)
        })
        .collect::<Vec<SubtaskResult>>()
}

fn document_to_evaluation_result_struct(submission_doc: Document) -> EvaluationResult {
    let compilation_result_struct = compilation_doc_to_struct(
        submission_doc.get("compilation")
            .expect(expected_field("compilation").as_str())
            .as_document()
            .unwrap()
        );
    let compilation_succeeded = compilation_result_struct.outcome == 1i32;
    EvaluationResult {
        compilation_result: compilation_result_struct,
        subtask_results: 
            if compilation_succeeded {
                subtasks_db_to_struct(
                    submission_doc.get("evaluation")
                        .expect(expected_field("evaluation").as_str())
                        .as_document()
                        .unwrap()

                )
            } else {
                vec![]
            },
        score: 
            score_option_bson_to_struct(submission_doc.get("overallScore"), true, expected_field("overallScore")) // expected
    }

}

fn generate_testcase_result() -> TestcaseResult {
    let mut gen = rand::thread_rng();
    let outcome = gen.gen::<i32>().checked_abs().unwrap_or(0) % 6;
    TestcaseResult {
        outcome: outcome,
        score: if outcome == testcase_result::Outcome::Ok as i32 {
            OneOfScore { score: Some(one_of_score::Score::BoolScore(true)) }
        } else {
            OneOfScore { score: Some(one_of_score::Score::BoolScore(false)) }
        },
        used_resources: Resources { time: time_ns_to_duration(gen.gen()), memory_bytes: gen.gen() },
    }
}

fn generate_subtask_result() -> SubtaskResult {
    SubtaskResult {
        testcase_results: vec![
            generate_testcase_result(),
            generate_testcase_result(),
            generate_testcase_result(),
            generate_testcase_result(),
            generate_testcase_result()
        ],
        score: OneOfScore { score: Some(one_of_score::Score::DoubleScore(0f64)) } // TODO this must be calculated with the lib function
    }
}

#[tonic::async_trait]
impl Submission for SubmissionService {
    async fn evaluate_submission(
        &self, 
        request: Request<EvaluateSubmissionRequest>
    ) -> Result<Response<protos::service::submission::EvaluateSubmissionResponse>, Status> { 
        // TODO is grpcurl gives default value to proto required fields, when not specified
        let evaluate_submission_request = request.into_inner();
        let submission = evaluate_submission_request.sub;
        // 1) write into dabatase with Pending state
        
        let doc_filter = create_pending_submission_document(submission.clone());
        let id = doc_filter.get_i64("_id").unwrap();

        self
            .get_collection()
            .insert_one(doc_filter.clone(), None)
            .await
            .map_err(internal_error)?;

	    // 2) redirect request to the dispatcher and await response
        let mut mock_dispatcher = MockDispatcher::default();
        
        mock_dispatcher.evaluate_submission_set(
            EvaluateSubmissionResponse {
                res: EvaluationResult {
                    compilation_result: CompilationResult { 
                        outcome: compilation_result::Outcome::Success as i32,
                        used_resources: Resources { 
                            time: time_ns_to_duration(1), 
                            memory_bytes: 1u64 
                        },
                        error_message: None
                    },
                    subtask_results: vec![
                        generate_subtask_result(),
                        generate_subtask_result(),
                        generate_subtask_result(),
                        generate_subtask_result(),
                        generate_subtask_result(),
                        generate_subtask_result()
                    ],
                    score: OneOfScore { score: Some(one_of_score::Score::DoubleScore(0f64)) }, // TODO this must be calculated with the lib function
                },
            }
        );

        let evaluation_result = 
            match mock_dispatcher.evaluate_submission(
                Request::new(
                    protos::service::dispatcher::EvaluateSubmissionRequest {
                        sub: submission,
                    }
                )
            )
            .await {
                Ok(evaluated) => evaluated.into_inner().res,
                Err(err) => {
                    // update state of the submission in the database to ABORTED 
                    let mut doc_updated = doc_filter.clone();
                    doc_updated.insert("state", SubmissionState::Aborted as i32);
                    
                    // set overall_score to 0?

                    self
                        .get_collection()
                        .update_one(
                            doc_filter,
                            doc! { "$set": doc_updated },
                            None
                        )
                        .await
                        .map_err(internal_error)?;

                    return Err(err);
                }
            };

        // TODO invoke RPC of evaluation service for scoring details
        // from the Dispatcher I get the scores of the testcases

        // lib function for calculating subtask scores starting from the testcases' scores
        // lib function for calculating submission score starting from subtasks' scores 

	    // 3) write values returned by the dispatcher into database
	    //    changing the state to EVALUATED
        let mut doc_updated = doc_filter.clone();
        doc_updated.insert("state", SubmissionState::Evaluated as i32);
        insert_evaluation_data_into_document(&mut doc_updated, &evaluation_result);

        let modified_count = self
            .get_collection()
            .update_one(
                doc_filter, // not necessary, _id is already enough
                doc! { "$set": doc_updated },
                None
            )
            .await
            .map_err(internal_error)?
            .modified_count;
        // modified_count MUST be 1
        assert_eq!(modified_count, 1u64);

        Ok(Response::new(
            protos::service::submission::EvaluateSubmissionResponse {
                res: evaluation_result,
                submission_id: id as u64,
            }
        ))
    }

    /*
    get_submission_list --> based on the author, we keep an index on user
    */
    async fn get_submission_list(
        &self,
        request: Request<GetSubmissionListRequest>
    ) -> Result<Response<GetSubmissionListResponse>, Status> { 
        let list_request = request.into_inner();
        let opt_limit = list_request.limit.clone();
        let opt_user = list_request.user.clone();
        let opt_problem_id = list_request.problem_id.clone();

        let mut doc_filter = Document::new();
        if let Some(user) = opt_user {
            doc_filter.insert("user", user);
        }
        if let Some(problem_id) = opt_problem_id {
            doc_filter.insert("problemId", convert_to_i64(problem_id));
        }

        let submissions = self
            .get_collection()
            .find(
                doc_filter,
                FindOptions::builder().limit( 
                    opt_limit.map(|limit| limit as i64)
                ).build()
            )
            .await
            .map_err(internal_error)?
            .filter(|opt_submission| futures::future::ready(opt_submission.is_ok()))
            .map(|some_submission| { 
                match some_submission {
                    Ok(submission) => Some(get_item_from_doc(submission)),
                    Err(_) => None
                }
            })
            .filter(|opt_item| futures::future::ready(opt_item.is_some()))
            .map(|some_item| some_item.unwrap())
            .collect::<Vec<_>>()
            .await;

        Ok(Response::new(
            GetSubmissionListResponse {
                list: submissions
            }
        ))
    }

    async fn get_submission_details(
        &self,
        request: Request<GetSubmissionDetailsRequest>
    ) -> Result<Response<GetSubmissionDetailsResponse>, Status> {
        let submission_details_request = request.into_inner();
        let submission_id = submission_details_request.submission_id;
        let opt_document = self
            .get_collection()
            .find_one(
                doc! {
                    "_id": convert_to_i64(submission_id)
                }, 
                None
            )
            .await
            .map_err(internal_error)?;
        
        match opt_document {
            Some(document) => {
                let state = document.get_i32("state")
                    .expect("This should not happen. \'state\' is a required field in db");
                
                Ok(Response::new(
                    GetSubmissionDetailsResponse {
                        sub: evaluation::Submission {
                            user: document.get_str("user")
                                .expect(expected_field("user").as_str())
                                .to_string(),
                            problem_id: document.get_i64("problemId")
                                .expect(expected_field("problemId").as_str()) as u64,
                            source: common::Source {
                                lang:
                                    document.get_i32("programmingLanguage")
                                        .expect(expected_field("programmingLanguage").as_str()),
                                code: {
                                    let bson_source = document.get("source")
                                        .expect(expected_field("source").as_str());
                                    match bson_source {
                                        Bson::Binary(bin_data) => bin_data.bytes.clone(),
                                        _ => panic!("This should not happen. \'source\' must be stored as Bson::Binary"),
                                    } 
                                }
                            }
                        },
                        state: state,
                        res: if state == SubmissionState::Evaluated as i32 {
                                Some(document_to_evaluation_result_struct(document))
                            } else {
                                None
                            }
                    }
                ))
            },
            None => {
                Err(Status::new(
                    tonic::Code::NotFound,
                    "Submission id provided is not present in database"
                ))
            }
        }
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_local_address(Service::CONTEST).parse()?;
    let submission_service = SubmissionService::new().await?;

    println!("Starting submission server");
    Server::builder()
        .add_service(SubmissionServer::new(submission_service))
        .serve(addr)
        .await?;
    Ok(())
}