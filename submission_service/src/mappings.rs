//extern crate serde;

use serde::*;
use ::utils::{gen_uuid, mongo::*};
use mongodb::{Client, Database, bson::{Binary, Bson, Document, Timestamp, bson, doc, spec::{BinarySubtype, ElementType}, to_document}, options::{ClientOptions, CreateCollectionOptions, FindOptions, ValidationAction, ValidationLevel}};
use protos::{*, self, common::Resources, evaluation::{CompilationResult, EvaluationResult, SubtaskResult, TestcaseResult, compilation_result, testcase_result}, service::{dispatcher::{EvaluateSubmissionResponse, MockDispatcher}}};
use protos::service::submission::*;
use protos::scoring::*;
use protos::service::submission::submission_server::*;
use protos::service::dispatcher::dispatcher_server::*;
use protos::utils::*;
use tonic::{Request, Response, Status, transport::*};

// references:
// [1]: https://serde.rs/enum-number.html#serialize-enum-as-number

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionObj {

    _id: u64, // required
    user : String, // required
    problem_id: u64, // required
    created: mongodb::bson::Timestamp, // required
    source: Vec<u8>, // required
    programming_language: common::ProgrammingLanguage, // required [1]
    state: service::submission::SubmissionState, // required [1]
    
    compilation: Option<CompilationObj>,
    execution: Option<ExecutionObj>, // instead of evaluation
    overall_score: Option<one_of_score::Score>
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompilationObj {
    outcome: compilation_result::Outcome, // required [1]

    #[serde(flatten)]    
    resources: ResourceObj,

    error_message: Option<String> // optional
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExecutionObj {
    subtasks: Vec<SubtaskObj>, // required
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubtaskObj {
    testcases: Vec<TestcaseObj>, // required
    subtask_score: one_of_score::Score // required
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TestcaseObj {
    outcome: testcase_result::Outcome, // required [1]
    score: one_of_score::Score, // required
    
    #[serde(flatten)]    
    resources: ResourceObj // required
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceObj {
    time_ns: i64, // required 
    memory_b: i64, // required
}

fn get_submission() -> SubmissionObj {
    todo!()
}

fn main() {

    let sub = to_document(&get_submission());
}