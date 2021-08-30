use core::panic;
use std::{collections::HashMap, time::SystemTime};

use futures::stream::StreamExt;

use convert::mongo::*;

use ::utils::gen_uuid;
use mongodb::{Client, bson::{Binary, Bson, Document, bson, doc, spec::{BinarySubtype, ElementType}}, options::{ClientOptions, FindOptions}};
use protos::{*, self, common::Resources, evaluation::{CompilationResult, EvaluationResult, SubtaskResult, TestcaseResult, subtask_result}, prost_types::Duration, service::{dispatcher::MockDispatcher}};
use protos::service::submission::*;
use protos::service::submission::submission_server::*;
use protos::service::dispatcher::dispatcher_server::*;
use protos::utils::*;
use tonic::{Request, Response, Status, transport::*};

#[cfg(test)]
mod tests;

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

#[derive(Debug)]
pub struct SubmissionService {
    db_client: Client,
}

impl SubmissionService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            db_client: Client::with_options(ClientOptions::parse(CONNECTION_STRING).await?)?,
        })
    }

    fn get_collection(&self) -> mongodb::Collection<Document> {
        self.db_client
            .database("submissionsdb")
            .collection::<Document>("submissions")
    }
}

type ScoreOneof = get_submission_list_response::item::OverallScore;

fn get_item_from_doc(doc: Document) -> get_submission_list_response::Item {
    get_submission_list_response::Item{
        submission_id: doc.get_i64("_id").unwrap() as u64,
        user: doc.get_str("user").unwrap().to_string(),
        problem_id: doc.get_i64("problemId").unwrap() as u64,
        timestamp: timestamp_to_systime(doc.get_timestamp("created").unwrap()).into(),
        overall_score: {
            let opt_score = doc.get("overallScore");
            if let Some(score) = opt_score {
                match score.element_type() {
                    ElementType::Double => 
                        Some(ScoreOneof::DoubleScore(score.as_f64().unwrap())),
                    ElementType::Boolean =>
                        Some(ScoreOneof::BoolScore(score.as_bool().unwrap())),
                    _ => None
                }
            } else {
                None
            }
        }
    }
}

fn convert_to_i64(x: u64) -> i64 {
    x as i64
}

fn create_pending_submission_document(
    problem_id: u64,
    source_code: Vec<u8>
) -> Document {
        doc! {
            "_id": convert_to_i64(gen_uuid()),
            "problemId": problem_id as i64,
            "created": convert::mongo::systime_to_timestamp(SystemTime::now()),
            "source": Bson::Binary(Binary {
                    subtype: BinarySubtype::Generic,
                    bytes: source_code
            }),
            "state": "PENDING"
        }
}

fn duration_to_time_ns(duration: Duration) -> i64 {
    let mut normalized_duration = duration.clone();
    normalized_duration.normalize();
    i64::min(i32::MAX.into(), normalized_duration.seconds) * 1_000_000_000 + (normalized_duration.nanos as i64)
}

fn time_ns_to_duration(time_ns: i64) -> Duration {
    Duration { 
        seconds: time_ns / 1_000_000_000, 
        nanos: (time_ns % 1_000_000_000) as i32
    }
}

fn compilation_data_to_db_obj(compilation_result: CompilationResult) -> Bson {
    let outcomes = ["None", "Success", "Rejected", "Tle", "Mle", "Rte"];
    let mut db_obj_document = doc! {
        "outcome": outcomes[compilation_result.outcome as usize],
        "timeNs": duration_to_time_ns(compilation_result.used_resources.time),
        "memoryB": convert_to_i64(compilation_result.used_resources.memory_bytes),
    };
    if let Some(err_msg) = compilation_result.error_message {
        db_obj_document.insert("error", err_msg);
    }
    db_obj_document.into()
}

fn get_score(score: subtask_result::SubtaskScore) -> Bson {
    match score {
        subtask_result::SubtaskScore::BoolScore(bool_score) => 
            bson! (bool_score),
        subtask_result::SubtaskScore::DoubleScore(double_score) => 
            bson! (double_score)
    }
}

fn get_score_testcase(score: evaluation::testcase_result::Score) -> Bson {
    match score {
        evaluation::testcase_result::Score::BoolScore(bool_score) => 
            bson! (bool_score),
        evaluation::testcase_result::Score::DoubleScore(double_score) => 
            bson! (double_score)
    }
}

fn testcase_data_to_db_obj(testcase_data: &TestcaseResult) -> Bson {
    let outcomes = ["NONE", "OK", "TLE", "MLE", "RTE", "CHECKER_ERROR"];
    bson! ({
        "outcome": outcomes[testcase_data.outcome as usize],
        "score": 
            get_score_testcase(
                testcase_data.score
                    .clone()
                    .expect("This should not happen. If compilation succeeds, then every testcase has a score")
            ),
        "timeNs": duration_to_time_ns(testcase_data.used_resources.time.clone()),
        "memoryB": convert_to_i64(testcase_data.used_resources.memory_bytes)
    })
}

fn subtask_data_to_db_obj(subtask_data: &SubtaskResult) -> Bson {
    bson! ({
        "subtaskScore": get_score(subtask_data.subtask_score.clone().expect(expected_field("subtaskScore").as_str())),
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
        if evaluation_result.compilation_result.outcome == 1 {
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
            // the Dispatcher MUST guarantee that the overall score is present even
            // if for some testcases bad stuff happened
            let score = evaluation_result.overall_score.clone()
                    .expect("This should not happen. Dispatcher should guarantee 
                            that score exists if compilation succeeded.");
            
            match score {
                evaluation::evaluation_result::OverallScore::BoolScore(bool_score) => 
                    doc_updated.insert("overallScore", bool_score),
                evaluation::evaluation_result::OverallScore::DoubleScore(double_score) => 
                    doc_updated.insert("overallScore", double_score)
            };
        }
}

fn compilation_doc_to_struct(compilation_doc: &Document) -> CompilationResult {
    let mut compilation_outcome_str_to_i32: HashMap<&str, i32> = HashMap::new();
    compilation_outcome_str_to_i32.insert("NONE", 0);
    compilation_outcome_str_to_i32.insert("SUCCESS", 1);
    compilation_outcome_str_to_i32.insert("REJECTED", 2);
    compilation_outcome_str_to_i32.insert("TLE", 3);
    compilation_outcome_str_to_i32.insert("MLE", 4);
    compilation_outcome_str_to_i32.insert("RTE", 5);
    
    CompilationResult {
        outcome: compilation_outcome_str_to_i32[compilation_doc.get_str("outcome").expect(expected_field("compilation").as_str())],
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
    let mut testcase_outcome_str_to_i32: HashMap<&str, i32> = HashMap::new();
    testcase_outcome_str_to_i32.insert("NONE", 0);
    testcase_outcome_str_to_i32.insert("OK", 1);
    testcase_outcome_str_to_i32.insert("TLE", 2);
    testcase_outcome_str_to_i32.insert("MLE", 3);
    testcase_outcome_str_to_i32.insert("RTE", 4);
    testcase_outcome_str_to_i32.insert("CHECKER_ERROR", 5);
    
    TestcaseResult {
        outcome: testcase_outcome_str_to_i32[
            testcase_doc.get_str("outcome").expect(expected_field("outcome").as_str())
        ],
        used_resources: Resources {
            time: time_ns_to_duration(testcase_doc.get_i64("timeNs").expect(expected_field("timeNs").as_str())),
            memory_bytes: testcase_doc.get_i64("memoryB").expect(expected_field("memoryB").as_str()) as u64,
        },
        score: {
            let score_bson = testcase_doc.get("score").expect(expected_field("score").as_str());
            match score_bson.element_type() {
                ElementType::Boolean => 
                    Some(evaluation::testcase_result::Score::BoolScore(score_bson.as_bool().unwrap())),
                ElementType::Double => 
                    Some(evaluation::testcase_result::Score::DoubleScore(score_bson.as_f64().unwrap())),
                _ => panic! ("score cannot have this type")
            }
        },
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
        subtask_score: 
            match subtask_score_bson.element_type() {
                ElementType::Boolean => 
                    Some(evaluation::subtask_result::SubtaskScore::BoolScore(subtask_score_bson.as_bool().unwrap())),
                ElementType::Double => 
                    Some(evaluation::subtask_result::SubtaskScore::DoubleScore(subtask_score_bson.as_f64().unwrap())),
                _ => panic! ("subtaskScore cannot have this type")
            },
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
        overall_score: {
            let score_bson = submission_doc.get("overallScore").expect(expected_field("overallScore").as_str());
            match score_bson.element_type() {
                ElementType::Boolean => 
                    Some(evaluation::evaluation_result::OverallScore::BoolScore(score_bson.as_bool().unwrap())),
                ElementType::Double => 
                    Some(evaluation::evaluation_result::OverallScore::DoubleScore(score_bson.as_f64().unwrap())),
                _ => panic! ("subtaskScore cannot have this type")
            }
        }
    }

}

#[tonic::async_trait]
impl Submission for SubmissionService {
    async fn evaluate_submission(
        &self, 
        request: Request<EvaluateSubmissionRequest>
    ) -> Result<Response<EvaluateSubmissionResponse>, Status> { 
        let evaluate_submission_request = request.into_inner();
        let submission = evaluate_submission_request.sub;
        // 1) write into dabatase with Pending state
        
        let doc_filter = create_pending_submission_document(
            submission.problem_id, 
            submission.source.code.clone()
        );
        let id = doc_filter.get_i64("_id").unwrap();

        self
            .get_collection()
            .insert_one(doc_filter.clone(), None)
            .await
            .map_err(internal_error)?;

	    // 2) redirect request to the dispatcher and await response
        let mock_dispatcher = MockDispatcher::default();
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
                    doc_updated.insert("state", "ABORTED");

                    self
                        .get_collection()
                        .update_one(
                            doc_filter,
                            doc_updated,
                            None
                        )
                        .await
                        .map_err(internal_error)?;

                    return Err(err);
                }
            };

	    // 3) write values returned by the dispatcher into database
	    //    changing the state to EVALUATED
        let mut doc_updated = doc_filter.clone();
        doc_updated.insert("state", "EVALUATED");
        insert_evaluation_data_into_document(&mut doc_updated, &evaluation_result);

        let modified_count = self
            .get_collection()
            .update_one(
                doc_filter, // not necessary, _id is already enough
                doc_updated,
                None
            )
            .await
            .map_err(internal_error)?
            .modified_count;
        // modified_count MUST be 1
        assert_eq!(modified_count, 1u64);

        Ok(Response::new(
            EvaluateSubmissionResponse {
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
        let mut state_str_to_i32: HashMap<&str, i32> = HashMap::new();
        state_str_to_i32.insert("PENDING", 0);
        state_str_to_i32.insert("EVALUATED", 1);
        state_str_to_i32.insert("ABORTED", 2);

        let mut prog_lang_str_to_i32: HashMap<&str, i32> = HashMap::new();
        prog_lang_str_to_i32.insert("RUST", 0);
        prog_lang_str_to_i32.insert("CPP", 1);

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
                let state_str = document.get_str("state")
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
                                lang: prog_lang_str_to_i32[
                                    document.get_str("programmingLanguage")
                                        .expect(expected_field("programmingLanguage").as_str())
                                ],
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
                        state: state_str_to_i32[state_str],
                        res: if state_str == "EVALUATED" {
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