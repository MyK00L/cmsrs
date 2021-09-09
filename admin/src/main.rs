// use chrono::TimeZone;
use protos::service::contest::contest_server::Contest;
use protos::service::evaluation::evaluation_server::Evaluation;
use protos::service::submission::submission_server::Submission;
use protos::service::{contest, evaluation, submission};
use rocket::form::{Form, Strict};
use rocket::fs::{relative, NamedFile};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::outcome::IntoOutcome;
use rocket::request::FromRequest;
use rocket::response::{status, Redirect};
use rocket::serde::Serialize;
use rocket::*;
use rocket_dyn_templates::Template;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use utils::gen_uuid;

const PASS: &str = "1234";
type ContestClient = contest::MockContest;
type SubmissionClient = submission::MockSubmission;
type EvaluationClient = evaluation::MockEvaluation;

// TODO: fix pub
pub struct Admin {}
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Admin {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Admin, ()> {
        request
            .cookies()
            .get_private("admin")
            .map(|_cookie| Admin {})
            .or_forward(())
    }
}

mod forms;
mod templates;

// static

#[get("/")]
async fn root_logged(_admin: Admin) -> Redirect {
    Redirect::to(uri!(templates::submissions_template))
}
#[get("/", rank = 2)]
async fn root() -> Option<NamedFile> {
    let path = Path::new(relative!("static/login/index.html"));
    NamedFile::open(path).await.ok()
}

#[rocket::get("/<path..>", rank = 7)]
async fn statics(_admin: Admin, path: PathBuf) -> Option<NamedFile> {
    let mut path = Path::new(relative!("static")).join(path);
    if path.is_dir() {
        path.push("index.html");
    }
    NamedFile::open(path).await.ok()
}
#[rocket::get("/<_path..>", rank = 8)]
async fn statics_redirect(_path: PathBuf) -> Redirect {
    Redirect::to(uri!(root))
}

// main
#[launch]
fn rocket() -> _ {
    let mut contest_client = contest::MockContest::default();
    let mut submission_client = submission::MockSubmission::default();
    let mut evaluation_client = evaluation::MockEvaluation::default();
    evaluation_client.get_contest_set(evaluation::GetContestResponse::default());
    submission_client.get_submission_list_set(submission::GetSubmissionListResponse {
        list: vec![submission::get_submission_list_response::Item {
            submission_id: 42,
            user: String::from("pippo"),
            problem_id: 2,
            state: submission::SubmissionState::Evaluated as i32,
            timestamp: SystemTime::now().into(),
            score: protos::scoring::OneOfScore {
                score: Some(protos::scoring::one_of_score::Score::DoubleScore(42.69)),
            },
        }],
    });
    submission_client.get_submission_details_set(submission::GetSubmissionDetailsResponse {
        sub: protos::evaluation::Submission {
            user: String::from("pippo"),
            problem_id: 2,
            source: protos::common::Source {
                code: "#define OII\nint main(){\n\treturn 0;\n}"
                    .as_bytes()
                    .to_vec(),
                lang: protos::common::ProgrammingLanguage::Rust as i32,
            },
        },
        state: protos::service::submission::SubmissionState::Evaluated as i32,
        res: Some(protos::evaluation::EvaluationResult {
            compilation_result: protos::evaluation::CompilationResult {
                outcome: protos::evaluation::compilation_result::Outcome::Success as i32,
                ..Default::default()
            },
            subtask_results: vec![
                protos::evaluation::SubtaskResult {
                    testcase_results: vec![
                        protos::evaluation::TestcaseResult {
                            outcome: protos::evaluation::testcase_result::Outcome::Ok as i32,
                            score: protos::scoring::OneOfScore {
                                score: Some(protos::scoring::one_of_score::Score::BoolScore(true))
                            },
                            ..Default::default()
                        };
                        9
                    ],
                    score: protos::scoring::OneOfScore {
                        score: Some(protos::scoring::one_of_score::Score::DoubleScore(42.69))
                    }
                };
                5
            ],
            score: protos::scoring::OneOfScore {
                score: Some(protos::scoring::one_of_score::Score::DoubleScore(3.3)),
            },
        }),
    });
    contest_client.add_message_set(contest::AddMessageResponse::default());
    contest_client.get_contest_metadata_set(contest::GetContestMetadataResponse {
        metadata: contest::ContestMetadata {
            name: String::from("contest"),
            description: String::from("wow awesome contest"),
            start_time: Some(SystemTime::now().into()),
            end_time: None,
        },
    });
    contest_client.get_question_list_set(contest::GetQuestionListResponse {
        questions: vec![
            contest::Message {
                subject: String::from("Problem A"),
                text: String::from("<b>hello</b>"),
                from: Some(String::from("me")),
                sent_at: SystemTime::now().into(),
                ..Default::default()
            },
            contest::Message {
                subject: String::from("Problem AA"),
                text: String::from("contains\nproblem\nid"),
                from: Some(String::from("a")),
                problem_id: Some(42),
                sent_at: SystemTime::now().into(),
                ..Default::default()
            },
        ],
    });
    rocket::build()
        .manage(contest_client)
        .manage(submission_client)
        .manage(evaluation_client)
        .mount(
            "/",
            routes![
                root,
                root_logged,
                statics,
                statics_redirect,
                templates::users_template,
                templates::questions_template,
                templates::submissions_template,
                templates::submission_details_template,
                templates::contest_template,
                forms::update_contest,
                forms::reply,
                forms::set_user,
                forms::login
            ],
        )
        .attach(Template::fairing())
}
