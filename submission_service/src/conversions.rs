use std::time::SystemTime;

use ::utils::{gen_uuid, mongo::*};
use mongodb::bson::{
    bson, doc,
    spec::{BinarySubtype, ElementType},
    Binary, Bson, Document,
};
use protos::scoring::{one_of_score, OneOfScore};
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

const DUMMY_MESSAGE: String = String::new();

fn expected_field(field_name: &str) -> String {
    format!(
        "This should not happen. In this context {} is a required field in db",
        field_name
    )
}

fn convert_to_i64(x: u64) -> i64 {
    x as i64
}

fn score_option_bson_to_struct(
    opt_bson_score: Option<&Bson>,
    expected: bool,
    expect_message: String,
) -> OneOfScore {
    OneOfScore {
        score: if let Some(bson_score) = opt_bson_score {
            match bson_score.element_type() {
                ElementType::Double => Some(one_of_score::Score::DoubleScore(
                    bson_score.as_f64().unwrap(),
                )),
                ElementType::Boolean => Some(one_of_score::Score::BoolScore(
                    bson_score.as_bool().unwrap(),
                )),
                _ => panic!("score cannot have this type"),
            }
        } else if expected {
            panic!("{}", expect_message.as_str())
        } else {
            None
        },
    }
}

fn score_struct_to_bson(score_struct: OneOfScore) -> Option<Bson> {
    match score_struct.score {
        Some(one_of_score::Score::DoubleScore(double_score)) => Some(Bson::Double(double_score)),
        Some(one_of_score::Score::BoolScore(bool_score)) => Some(Bson::Boolean(bool_score)),
        _ => None,
    }
}

pub fn get_item_from_doc(doc: Document) -> get_submission_list_response::Item {
    get_submission_list_response::Item {
        submission_id: doc.get_i64("_id").unwrap() as u64,
        user: doc.get_str("user").unwrap().to_string(),
        problem_id: doc.get_i64("problemId").unwrap() as u64,
        timestamp: timestamp_to_systime(doc.get_timestamp("created").unwrap()).into(),
        state: doc
            .get_i32("state")
            .expect(expected_field("state").as_str()),
        score: score_option_bson_to_struct(doc.get("overallScore"), false, DUMMY_MESSAGE),
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
        "subtaskScore": score_struct_to_bson(subtask_data.score.clone()),
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
    if evaluation_result.compilation_result.outcome == compilation_result::Outcome::Success as i32 {
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

        doc_updated.insert(
            "overallScore",
            score_struct_to_bson(evaluation_result.score.clone())
                .expect("Score should have just been evaluated."),
        );
    } else {
        doc_updated.insert("overallScore", Bson::Double(0f64));
    }
}

fn compilation_doc_to_struct(compilation_doc: &Document) -> CompilationResult {
    CompilationResult {
        outcome: compilation_doc
            .get_i32("outcome")
            .expect(expected_field("compilation").as_str()),
        used_resources: Resources {
            time: time_ns_to_duration(
                compilation_doc
                    .get_i64("timeNs")
                    .expect(expected_field("timeNs").as_str()),
            ),
            memory_bytes: compilation_doc
                .get_i64("memoryB")
                .expect(expected_field("memoryB").as_str()) as u64,
        },
        error_message: compilation_doc.get("error").map(|bson_string| {
            bson_string
                .as_str()
                .expect("This should not happen. \'error\' must be stored as Bson::String")
                .to_string()
        }),
    }
}

fn single_testcase_db_to_struct(testcase_doc: &Document) -> TestcaseResult {
    TestcaseResult {
        outcome: testcase_doc
            .get_i32("outcome")
            .expect(expected_field("outcome").as_str()),
        used_resources: Resources {
            time: time_ns_to_duration(
                testcase_doc
                    .get_i64("timeNs")
                    .expect(expected_field("timeNs").as_str()),
            ),
            memory_bytes: testcase_doc
                .get_i64("memoryB")
                .expect(expected_field("memoryB").as_str()) as u64,
        },
        score: score_option_bson_to_struct(
            testcase_doc.get("score"),
            true,
            expected_field("score"),
        ), // expected
    }
}

fn single_subtask_db_to_struct(subtask_doc: &Document) -> SubtaskResult {
    let subtask_score_bson = subtask_doc
        .get("subtaskScore")
        .expect(expected_field("subtaskScore").as_str());

    SubtaskResult {
        testcase_results: subtask_doc
            .get_array("testcases")
            .expect(expected_field("testcases").as_str())
            .iter()
            .map(|bson_testcase| {
                let testcase = bson_testcase.as_document().unwrap();
                single_testcase_db_to_struct(testcase)
            })
            .collect::<Vec<TestcaseResult>>(),
        score: score_option_bson_to_struct(Some(subtask_score_bson), false, DUMMY_MESSAGE),
    }
}

fn subtasks_db_to_struct(evaluation_doc: &Document) -> Vec<SubtaskResult> {
    evaluation_doc
        .get_array("subtasks")
        .expect(expected_field("subtasks").as_str())
        .iter()
        .map(|bson_subtask| {
            let subtask = bson_subtask.as_document().unwrap();
            single_subtask_db_to_struct(subtask)
        })
        .collect::<Vec<SubtaskResult>>()
}

pub fn document_to_evaluation_result_struct(submission_doc: Document) -> EvaluationResult {
    let compilation_result_struct = compilation_doc_to_struct(
        submission_doc
            .get("compilation")
            .expect(expected_field("compilation").as_str())
            .as_document()
            .unwrap(),
    );
    let compilation_succeeded = compilation_result_struct.outcome == 1i32;
    EvaluationResult {
        compilation_result: compilation_result_struct,
        subtask_results: if compilation_succeeded {
            subtasks_db_to_struct(
                submission_doc
                    .get("evaluation")
                    .expect(expected_field("evaluation").as_str())
                    .as_document()
                    .unwrap(),
            )
        } else {
            vec![]
        },
        score: score_option_bson_to_struct(
            submission_doc.get("overallScore"),
            true,
            expected_field("overallScore"),
        ), // expected
    }
}
