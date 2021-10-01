use std::time::SystemTime;

use ::utils::{gen_uuid, mongo::*};
use mongodb::bson::{bson, doc, spec::BinarySubtype, Binary, Bson, Document};
use protos::common::Score;
use protos::service::submission::*;
use protos::utils::*;
use protos::{
    self,
    common::Resources,
    evaluation::{
        compilation_result, CompilationResult, EvaluationResult, SubtaskResult, TestcaseResult,
    },
    *,
};

fn expected_field(field_name: &str) -> String {
    format!(
        "This should not happen. In this context {} is a required field in db",
        field_name
    )
}

fn convert_to_i64(x: u64) -> i64 {
    x as i64
}

pub fn get_item_from_doc(doc: Document) -> get_submission_list_response::Item {
    get_submission_list_response::Item {
        submission_id: doc.get_i64("_id").unwrap() as u64,
        user: doc.get_str("user").unwrap().to_string(),
        problem_id: doc.get_i64("problemId").unwrap() as u64,
        timestamp: timestamp_to_systime(doc.get_timestamp("created").unwrap()).into(),
        state: doc
            .get_i32("state")
            .unwrap_or_else(|_| panic!("{}", expected_field("state"))),
        score: doc
            .get_f64("overallScore")
            .map_or_else(|_| None, |val| Some(Score { score: val })),
    }
}

pub fn create_pending_submission_document(submission: evaluation::Submission) -> Document {
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
        (duration.secs as i64) * 1_000_000_000 + (duration.nanos as i64)
    }
}

fn time_ns_to_duration(time_ns: i64) -> common::Duration {
    common::Duration {
        secs: (time_ns / 1_000_000_000) as u64,
        nanos: (time_ns % 1_000_000_000) as u32,
    }
}

fn compilation_data_to_db_obj(compilation_result: CompilationResult) -> Bson {
    bson! ({
        "outcome": compilation_result.outcome,
        "timeNs": duration_to_time_ns(compilation_result.used_resources.time),
        "memoryB": convert_to_i64(compilation_result.used_resources.memory_bytes)
    })
}

fn testcase_data_to_db_obj(testcase_data: &TestcaseResult) -> Bson {
    bson! ({
        "testcaseId": convert_to_i64(testcase_data.id),
        "outcome": testcase_data.outcome,
        "score": testcase_data.score.score,
        "timeNs": duration_to_time_ns(testcase_data.used_resources.time.clone()),
        "memoryB": convert_to_i64(testcase_data.used_resources.memory_bytes)
    })
}

fn subtask_data_to_db_obj(subtask_data: &SubtaskResult) -> Bson {
    bson! ({
        "subtaskId": convert_to_i64(subtask_data.id),
        "subtaskScore": subtask_data.score.score,
        "testcases":
            subtask_data.testcase_results
                .iter()
                .map(|testcase_data| testcase_data_to_db_obj(testcase_data))
                .collect::<Bson>()
    })
}

pub fn insert_evaluation_data_into_document(
    doc_updated: &mut Document,
    evaluation_result: &EvaluationResult,
) {
    doc_updated.insert(
        "compilation",
        compilation_data_to_db_obj(evaluation_result.compilation_result.clone()),
    );
    doc_updated.insert("overallScore", evaluation_result.score.score);

    if evaluation_result.compilation_result.outcome() == compilation_result::Outcome::Success {
        // if compilation succeeded, then fill evaluation and score fields
        doc_updated.insert(
            "evaluation",
            bson! ({
                "subtasks":
                    evaluation_result.subtask_results
                        .clone()
                        .iter()
                        .map(|subtask_data| {
                            subtask_data_to_db_obj(subtask_data)
                        })
                        .collect::<Bson>()
            }),
        );
    }
}

fn compilation_doc_to_struct(compilation_doc: &Document) -> CompilationResult {
    CompilationResult {
        outcome: compilation_doc
            .get_i32("outcome")
            .unwrap_or_else(|_| panic!("{}", expected_field("outcome"))),
        used_resources: Resources {
            time: time_ns_to_duration(
                compilation_doc
                    .get_i64("timeNs")
                    .unwrap_or_else(|_| panic!("{}", expected_field("timeNs"))),
            ),
            memory_bytes: compilation_doc
                .get_i64("memoryB")
                .unwrap_or_else(|_| panic!("{}", expected_field("memoryB")))
                as u64,
        },
    }
}

fn single_testcase_db_to_struct(testcase_doc: &Document) -> TestcaseResult {
    TestcaseResult {
        outcome: testcase_doc
            .get_i32("outcome")
            .unwrap_or_else(|_| panic!("{}", expected_field("outcome"))),
        used_resources: Resources {
            time: time_ns_to_duration(
                testcase_doc
                    .get_i64("timeNs")
                    .unwrap_or_else(|_| panic!("{}", expected_field("timeNs"))),
            ),
            memory_bytes: testcase_doc
                .get_i64("memoryB")
                .unwrap_or_else(|_| panic!("{}", expected_field("memoryB")))
                as u64,
        },
        score: Score {
            score: testcase_doc
                .get_f64("score")
                .unwrap_or_else(|_| panic!("{}", expected_field("score"))),
        },
        id: testcase_doc
            .get_i64("testcaseId")
            .unwrap_or_else(|_| panic!("{}", expected_field("testcaseId"))) as u64,
    }
}

fn single_subtask_db_to_struct(subtask_doc: &Document) -> SubtaskResult {
    SubtaskResult {
        testcase_results: subtask_doc
            .get_array("testcases")
            .unwrap_or_else(|_| panic!("{}", expected_field("testcases")))
            .iter()
            .map(|bson_testcase| {
                let testcase = bson_testcase.as_document().unwrap();
                single_testcase_db_to_struct(testcase)
            })
            .collect::<Vec<TestcaseResult>>(),
        score: Score {
            score: subtask_doc
                .get_f64("subtaskScore")
                .unwrap_or_else(|_| panic!("{}", expected_field("subtaskScore"))),
        },
        id: subtask_doc
            .get_i64("subtaskId")
            .unwrap_or_else(|_| panic!("{}", expected_field("subtaskId"))) as u64,
    }
}

fn subtasks_db_to_struct(evaluation_doc: &Document) -> Vec<SubtaskResult> {
    evaluation_doc
        .get_array("subtasks")
        .unwrap_or_else(|_| panic!("{}", expected_field("subtasks")))
        .iter()
        .map(|bson_subtask| {
            let subtask = bson_subtask.as_document().unwrap();
            single_subtask_db_to_struct(subtask)
        })
        .collect::<Vec<SubtaskResult>>()
}

/// Pre: state is Evaluated
pub fn document_to_evaluation_result_struct(submission_doc: Document) -> EvaluationResult {
    let compilation_result_struct = compilation_doc_to_struct(
        submission_doc
            .get("compilation")
            .unwrap_or_else(|| panic!("{}", expected_field("compilation")))
            .as_document()
            .unwrap(),
    );
    let compilation_succeeded =
        compilation_result_struct.outcome() == compilation_result::Outcome::Success;
    EvaluationResult {
        compilation_result: compilation_result_struct,
        subtask_results: if compilation_succeeded {
            subtasks_db_to_struct(
                submission_doc
                    .get("evaluation")
                    .unwrap_or_else(|| panic!("{}", expected_field("evaluation")))
                    .as_document()
                    .unwrap(),
            )
        } else {
            vec![]
        },
        score: Score {
            score: submission_doc
                .get_f64("overallScore")
                .unwrap_or_else(|_| panic!("{}", expected_field("overallScore"))),
        },
    }
}
