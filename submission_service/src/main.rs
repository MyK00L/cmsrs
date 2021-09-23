#![recursion_limit = "256"]

use core::panic;

use futures::stream::StreamExt;

use ::utils::scoring_lib::{evaluate_submission_score, evaluate_subtask_score, score_with_double};
use mongodb::{
    bson::{doc, Bson, Document},
    options::{
        ClientOptions, CreateCollectionOptions, FindOptions, ValidationAction, ValidationLevel,
    },
    Client, Database,
};
use protos::service::submission::submission_server::*;
use protos::service::submission::*;
use protos::utils::*;
use protos::{self, *};
use protos::{
    evaluation::compilation_result,
    service::evaluation::{evaluation_server::Evaluation, GetProblemRequest},
};
use protos::{evaluation::EvaluationResult, service::dispatcher::dispatcher_server::*};
use tonic::{transport::*, Request, Response, Status};

mod conversions;

mod mock_services;

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
    format!(
        "This should not happen. In this context, {} is a required field in db",
        field_name
    )
}

fn convert_to_i64(x: u64) -> i64 {
    x as i64
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
                                    "enum": [0, 1, 2, 3, 4]
                                    /*
                                    0 => NONE
                                    1 => SUCCESS
                                    2 => TLE
                                    3 => MLE
                                    4 => RTE
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
                                    "required": ["subtaskId", "testcases", "subtaskScore"],
                                    "properties": {
                                    "subtaskId": { "bsonType": "long" },
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
                                            "required": ["testcaseId", "outcome", "score", "timeNs", "memoryB"], 
                                            "properties": {
                                                "testcaseId": { "bsonType": "long" },
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
        Ok(Self { db_client })
    }

    fn get_collection(&self) -> mongodb::Collection<Document> {
        self.db_client
            .database("submissionsdb")
            .collection::<Document>("submissions")
    }
}

/// safe even if compilation didn't succeed
async fn evaluate_scores(
    mut_evaluation_result: &mut EvaluationResult,
    problem_id: u64,
) -> Result<(), Status> {
    // if compilation failed, update manually submission score and return
    if let compilation_result::Outcome::Success = mut_evaluation_result.compilation_result.outcome()
    {
        assert!(mut_evaluation_result.subtask_results.is_empty());
        mut_evaluation_result.score = score_with_double(0f64);
        return Ok(());
    }

    let problem_metadata_request = GetProblemRequest { problem_id };
    let mock_evaluation_server = mock_services::get_mock_evaluation(problem_id);

    let problem_metadata = mock_evaluation_server
        .get_problem(Request::new(problem_metadata_request))
        .await?
        .into_inner();

    mut_evaluation_result
        .subtask_results
        .iter_mut()
        .enumerate()
        .for_each(|(i, subtask)| {
            evaluate_subtask_score(
                &subtask.testcase_results,
                &problem_metadata.info.subtasks[i].scoring,
                &mut subtask.score,
            );
        });

    evaluate_submission_score(
        &mut_evaluation_result.subtask_results,
        &mut mut_evaluation_result.score,
    );

    Ok(())
}

#[tonic::async_trait]
impl Submission for SubmissionService {
    async fn evaluate_submission(
        &self,
        request: Request<EvaluateSubmissionRequest>,
    ) -> Result<Response<protos::service::submission::EvaluateSubmissionResponse>, Status> {
        let evaluate_submission_request = request.into_inner();
        let submission = evaluate_submission_request.sub;
        // 1) write into dabatase with Pending state

        let doc_filter = conversions::create_pending_submission_document(submission.clone());
        let id = doc_filter.get_i64("_id").unwrap();

        self.get_collection()
            .insert_one(doc_filter.clone(), None)
            .await
            .map_err(internal_error)?;

        // 2) redirect request to the dispatcher and await response
        let mock_dispatcher = mock_services::get_mock_dispatcher();

        let evaluation_result = match mock_dispatcher
            .evaluate_submission(Request::new(
                protos::service::dispatcher::EvaluateSubmissionRequest {
                    sub: submission.clone(),
                },
            ))
            .await
        {
            Ok(evaluated) => evaluated.into_inner().res,
            Err(err) => {
                // update state of the submission in the database to ABORTED
                let mut doc_updated = doc_filter.clone();
                doc_updated.insert("state", SubmissionState::Aborted as i32);

                doc_updated.insert("overallScore", 0f64);

                self.get_collection()
                    .update_one(doc_filter, doc! { "$set": doc_updated }, None)
                    .await
                    .map_err(internal_error)?;

                return Err(err);
            }
        };

        // evaluate subtasks' and submission's scores starting from testcases' scores
        // and problem metadata
        let mut mut_evaluation_result = evaluation_result.clone();
        evaluate_scores(&mut mut_evaluation_result, submission.problem_id).await?;

        // 3) write values returned by the dispatcher into database
        //    changing the state to EVALUATED
        let mut doc_updated = doc_filter.clone();
        doc_updated.insert("state", SubmissionState::Evaluated as i32);
        conversions::insert_evaluation_data_into_document(&mut doc_updated, &mut_evaluation_result);

        self.get_collection()
            .update_one(doc! { "_id": id }, doc! { "$set": doc_updated }, None)
            .await
            .map_err(internal_error)?;

        Ok(Response::new(
            protos::service::submission::EvaluateSubmissionResponse {
                res: mut_evaluation_result,
                submission_id: id as u64,
            },
        ))
    }

    async fn get_submission_list(
        &self,
        request: Request<GetSubmissionListRequest>,
    ) -> Result<Response<GetSubmissionListResponse>, Status> {
        let list_request = request.into_inner();
        let opt_limit = list_request.limit;
        let opt_user = list_request.user.clone();
        let opt_problem_id = list_request.problem_id;

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
                FindOptions::builder()
                    .limit(opt_limit.map(|limit| limit as i64))
                    .build(),
            )
            .await
            .map_err(internal_error)?
            .filter(|opt_submission| futures::future::ready(opt_submission.is_ok()))
            .map(|some_submission| match some_submission {
                Ok(submission) => Some(conversions::get_item_from_doc(submission)),
                Err(_) => None,
            })
            .filter(|opt_item| futures::future::ready(opt_item.is_some()))
            .map(|some_item| some_item.unwrap())
            .collect::<Vec<_>>()
            .await;

        Ok(Response::new(GetSubmissionListResponse {
            list: submissions,
        }))
    }

    async fn get_submission_details(
        &self,
        request: Request<GetSubmissionDetailsRequest>,
    ) -> Result<Response<GetSubmissionDetailsResponse>, Status> {
        let submission_details_request = request.into_inner();
        let submission_id = submission_details_request.submission_id;
        let opt_document = self
            .get_collection()
            .find_one(doc! { "_id": convert_to_i64(submission_id) }, None)
            .await
            .map_err(internal_error)?;

        // consider using opt_document.map_or_else(default, f) instead of the match expression
        match opt_document {
            Some(document) => {
                let state = document
                    .get_i32("state")
                    .unwrap_or_else(|_| panic!("{}", expected_field("problemId")));

                Ok(Response::new(GetSubmissionDetailsResponse {
                    sub: evaluation::Submission {
                        user: document
                            .get_str("user")
                            .unwrap_or_else(|_| panic!("{}", expected_field("user")))
                            .to_string(),
                        problem_id: document
                            .get_i64("problemId")
                            .unwrap_or_else(|_| panic!("{}", expected_field("problemId")))
                            as u64,
                        source: common::Source {
                            lang: document.get_i32("programmingLanguage").unwrap_or_else(|_| {
                                panic!("{}", expected_field("programmingLanguage"))
                            }),
                            code: {
                                let bson_source = document
                                    .get("source")
                                    .unwrap_or_else(|| panic!("{}", expected_field("source")));
                                match bson_source {
                                        Bson::Binary(bin_data) => bin_data.bytes.clone(),
                                        _ => panic!("This should not happen. \'source\' must be stored as Bson::Binary"),
                                    }
                            },
                        },
                    },
                    state,
                    res: if state == SubmissionState::Evaluated as i32 {
                        Some(conversions::document_to_evaluation_result_struct(document))
                    } else {
                        None
                    },
                }))
            }
            None => Err(Status::new(
                tonic::Code::NotFound,
                "Submission id provided is not present in database",
            )),
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
