use chrono::TimeZone;
use futures::future;
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

#[cfg(feature = "mock")]
mod clients {
    pub use protos::service::contest::contest_server::Contest;
    pub use protos::service::evaluation::evaluation_server::Evaluation;
    pub use protos::service::submission::submission_server::Submission;
    use protos::service::{contest, evaluation, submission};
    // clients for testing
    pub type ContestClient = contest::MockContest;
    pub type SubmissionClient = submission::MockSubmission;
    pub type EvaluationClient = evaluation::MockEvaluation;
    use fake::{Fake, Faker};
    pub fn get_contest_client() -> ContestClient {
        let mut mock = contest::MockContest::default();

        mock.add_message_set(contest::AddMessageResponse::default());

        let mut ql: contest::GetQuestionListResponse = Faker.fake();
        for q in ql.questions.iter_mut() {
            q.sent_at = std::time::SystemTime::now().into();
        }
        mock.get_question_list_set(ql);

        let mut al: contest::GetAnnouncementListResponse = Faker.fake();
        for a in al.announcements.iter_mut() {
            a.sent_at = std::time::SystemTime::now().into();
        }
        mock.get_announcement_list_set(al);

        /*mock.get_problem_set(contest::GetProblemResponse {
            info: contest::Problem {
                id: 42,
                name: String::from("problemname"),
                long_name: String::from("Loooooong problem name"),
            },
            ..Default::default()
        });*/

        mock.get_contest_metadata_set(contest::GetContestMetadataResponse {
            metadata: contest::ContestMetadata {
                name: String::from("contest"),
                description: String::from("wow awesome contest"),
                start_time: Some(std::time::SystemTime::now().into()),
                end_time: None,
            },
            problems: Faker.fake(),
        });

        mock
    }
    pub fn get_submission_client() -> SubmissionClient {
        let mut mock = submission::MockSubmission::default();
        mock.get_submission_list_set(submission::GetSubmissionListResponse {
            list: vec![submission::get_submission_list_response::Item {
                submission_id: 42,
                user: String::from("pippo"),
                problem_id: 2,
                state: submission::SubmissionState::Evaluated as i32,
                timestamp: std::time::SystemTime::now().into(),
                score: Some(protos::common::Score { score: 42.69 }),
            }],
        });
        mock.get_submission_details_set(submission::GetSubmissionDetailsResponse {
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
                        id: 1234,
                        testcase_results: vec![
                            protos::evaluation::TestcaseResult {
                                outcome: protos::evaluation::testcase_result::Outcome::Ok as i32,
                                score: protos::common::Score { score: 1.0 },
                                ..Default::default()
                            };
                            9
                        ],
                        score: protos::common::Score { score: 1.0 },
                    };
                    5
                ],
                score: protos::common::Score { score: 3.3 },
            }),
        });
        mock
    }
    pub fn get_evaluation_client() -> EvaluationClient {
        let mut mock = evaluation::MockEvaluation::default();
        mock.get_contest_set(evaluation::GetContestResponse {
            info: evaluation::Contest {
                problems: vec![
                    evaluation::Problem {
                        id: 42,
                        subtasks: vec![
                            evaluation::Subtask {
                                id: 69,
                                testcases_id: vec![1, 2, 3, 4, 5, 6, 7, 9, 123],
                                ..Default::default()
                            };
                            5
                        ],
                        ..Default::default()
                    };
                    3
                ],
                user_scoring_method: protos::scoring::User {
                    main: protos::scoring::user::Method::default(),
                    tiebreakers: vec![protos::scoring::user::Method::default(); 4],
                },
            },
        });
        mock
    }
}

#[cfg(not(feature = "mock"))]
mod clients {
    use protos::service::{contest, evaluation, submission};
    // clients for production
    pub type ContestClient = contest::contest_client::ContestClient<tonic::transport::Channel>;
    pub type SubmissionClient =
        submission::submission_client::SubmissionClient<tonic::transport::Channel>;
    pub type EvaluationClient =
        evaluation::evaluation_client::EvaluationClient<tonic::transport::Channel>;
    pub fn get_contest_client() -> ContestClient {
        ContestClient::new(protos::utils::get_new_channel(
            protos::utils::Service::CONTEST,
        ))
    }
    pub fn get_submission_client() -> SubmissionClient {
        SubmissionClient::new(protos::utils::get_new_channel(
            protos::utils::Service::SUBMISSION,
        ))
    }
    pub fn get_evaluation_client() -> EvaluationClient {
        EvaluationClient::new(protos::utils::get_new_channel(
            protos::utils::Service::EVALUATION,
        ))
    }
}

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
    let contest_client = clients::get_contest_client();
    let submission_client = clients::get_submission_client();
    let evaluation_client = clients::get_evaluation_client();
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
                templates::problem_files_template,
                templates::questions_template,
                templates::submissions_template,
                templates::submission_details_template,
                templates::contest_template,
                forms::update_contest,
                forms::reply,
                forms::set_user,
                forms::login,
                forms::set_evaluation_file,
                forms::add_testcase,
                forms::get_evaluation_file,
                forms::get_testcase,
            ],
        )
        .attach(Template::fairing())
}
