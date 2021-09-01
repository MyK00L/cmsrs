use chrono::TimeZone;
use protos::service::contest::contest_server::Contest;
use protos::service::submission::submission_server::Submission;
use protos::service::{contest, submission};
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

struct Admin {}
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

// API (forms and stuff)

#[derive(FromForm)]
struct LoginForm {
    pass: String,
}
#[post("/form/login", data = "<login>")]
async fn login_form(cookies: &CookieJar<'_>, login: Form<Strict<LoginForm>>) -> Redirect {
    if login.pass == PASS {
        cookies.add_private(Cookie::new("admin", "admin"));
        Redirect::to("/home")
    } else {
        Redirect::to(uri!(root))
    }
}

#[derive(FromForm)]
struct UpdateContestForm {
    name: String,
    description: String,
    start_time: String,
    end_time: String,
}
#[post("/form/update_contest", data = "<contest>")]
async fn update_contest_form(
    _admin: Admin,
    contest: Form<Strict<UpdateContestForm>>,
    contest_client: &State<ContestClient>,
) -> Result<Redirect, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    let req = contest::SetContestMetadataRequest {
        metadata: contest::ContestMetadata {
            name: contest.name.clone(),
            description: contest.description.clone(),
            start_time: chrono::prelude::Utc
                .datetime_from_str(&contest.start_time, "%FT%T")
                .ok()
                .map(|t| SystemTime::from(t).into()),
            end_time: chrono::prelude::Utc
                .datetime_from_str(&contest.end_time, "%FT%T")
                .ok()
                .map(|t| SystemTime::from(t).into()),
        },
    };
    match contest_client
        .set_contest_metadata(tonic::Request::new(req))
        .await
    {
        Ok(_) => Ok(Redirect::to("/contest")),
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

#[derive(FromForm)]
struct AddUserForm {
    username: String,
    fullname: String,
    password: String,
}
#[post("/form/set_user", data = "<user>")]
async fn set_user_form(
    _admin: Admin,
    user: Form<Strict<AddUserForm>>,
    contest_client: &State<ContestClient>,
) -> Result<Redirect, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    let req = contest::SetUserRequest {
        username: user.username.clone(),
        fullname: user.fullname.clone(),
        password: user.password.clone(),
    };
    match contest_client.set_user(tonic::Request::new(req)).await {
        Ok(_) => Ok(Redirect::to("/users")),
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

#[derive(FromForm)]
struct ReplyForm {
    user: String,
    subject: String,
    problem_id: Option<u64>,
    text: String,
    broadcast: Option<bool>,
}
#[post("/form/reply", data = "<message>")]
async fn reply_form(
    _admin: Admin,
    message: Form<Strict<ReplyForm>>,
    contest_client: &State<ContestClient>,
) -> Result<Redirect, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    let req = contest::AddMessageRequest {
        message: contest::Message {
            id: gen_uuid(),
            subject: message.subject.clone(),
            problem_id: message.problem_id,
            text: message.text.clone(),
            to: if Some(true) == message.broadcast {
                None
            } else {
                Some(message.user.clone())
            },
            from: None,
            sent_at: SystemTime::now().into(),
        },
    };
    match contest_client.add_message(tonic::Request::new(req)).await {
        Ok(_) => Ok(Redirect::to(uri!(questions_template))),
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

// templates

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateTestcaseResult {
    verdict: String,
    outcome: String,
    millis: u64,
    bytes: u64,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubtaskResult {
    n: u64,
    score: String,
    testcase_results: Vec<TemplateTestcaseResult>,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubmissionEvaluation {
    compilation_outcome: String,
    compilation_millis: u64,
    compilation_bytes: u64,
    compilation_error: String,
    score: String,
    subtask_results: Vec<TemplateSubtaskResult>,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubmissionDetails {
    state: String,
    user: String,
    problem_id: u64,
    lang: String,
    code: String,
    evaluation: Option<TemplateSubmissionEvaluation>,
}
#[get("/submission/<id>")]
async fn submission_details_template(
    _admin: Admin,
    submission_client: &State<SubmissionClient>,
    id: u64,
) -> Result<Template, status::Custom<String>> {
    let submission_client = submission_client.inner().clone();
    match submission_client
        .get_submission_details(tonic::Request::new(
            submission::GetSubmissionDetailsRequest { submission_id: id },
        ))
        .await
    {
        Ok(response) => {
            let res = response.into_inner();
            let submission_details = TemplateSubmissionDetails {
                state: format!(
                    "{:?}",
                    submission::SubmissionState::from_i32(res.state).unwrap()
                ),
                user: res.sub.user.to_string(),
                problem_id: res.sub.problem_id,
                lang: format!(
                    "{:?}",
                    protos::common::ProgrammingLanguage::from_i32(res.sub.source.lang).unwrap()
                ),
                code: String::from_utf8(res.sub.source.code.clone())
                    .unwrap_or(format!("{:?}", res.sub.source.code)),
                evaluation: match res.res {
                    Some(res) => Some(TemplateSubmissionEvaluation {
                        compilation_outcome: format!(
                            "{:?}",
                            protos::evaluation::compilation_result::Outcome::from_i32(
                                res.compilation_result.outcome
                            )
                            .unwrap()
                        ),
                        compilation_millis: (res.compilation_result.used_resources.time.secs
                            * 1000
                            + res.compilation_result.used_resources.time.nanos as u64 / 1000000),
                        compilation_bytes: res.compilation_result.used_resources.memory_bytes,
                        compilation_error: res
                            .compilation_result
                            .error_message
                            .clone()
                            .unwrap_or_else(|| String::from("")),
                        score: format!("{:?}", res.score),
                        subtask_results: res
                            .subtask_results
                            .iter()
                            .map(|sr| TemplateSubtaskResult {
                                n: 0,
                                score: format!("{:?}", sr.score),
                                testcase_results: sr
                                    .testcase_results
                                    .iter()
                                    .map(|tr| TemplateTestcaseResult {
                                        verdict: format!("{:?}", tr.score),
                                        outcome: format!(
                                            "{:?}",
                                            protos::evaluation::testcase_result::Outcome::from_i32(
                                                tr.outcome
                                            )
                                            .unwrap()
                                        ),
                                        millis: (tr.used_resources.time.secs * 1000
                                            + res.compilation_result.used_resources.time.nanos
                                                as u64
                                                / 1000000),
                                        bytes: tr.used_resources.memory_bytes,
                                    })
                                    .collect(),
                            })
                            .collect(),
                    }),
                    None => None,
                },
            };
            Ok(Template::render("submission_details", submission_details))
        }
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateQuestion {
    id: u64,
    problem_id: Option<u64>,
    time: String,
    user: String,
    subject: String,
    text: String,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateQuestions {
    questions: Vec<TemplateQuestion>,
}
#[get("/questions")]
async fn questions_template(
    _admin: Admin,
    contest_client: &State<ContestClient>,
) -> Result<Template, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    match contest_client
        .get_question_list(tonic::Request::new(
            contest::GetQuestionListRequest::default(),
        ))
        .await
    {
        Ok(response) => {
            let questions = TemplateQuestions {
                questions: response
                    .into_inner()
                    .questions
                    .iter()
                    .map(|q| TemplateQuestion {
                        id: q.id,
                        problem_id: q.problem_id,
                        time: utils::render_protos_timestamp(q.sent_at.clone(), "%F %X"),
                        user: q.from.clone().unwrap_or_else(|| String::from("")),
                        subject: q.subject.clone(),
                        text: q.text.clone(),
                    })
                    .collect(),
            };
            Ok(Template::render("questions", questions))
        }
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubmissionsItem {
    submission_id: u64,
    problem_id: u64,
    user: String,
    state: String,
    time: String,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubmissions {
    submission_list: Vec<TemplateSubmissionsItem>,
}
#[get("/submissions")]
async fn submissions_template(
    _admin: Admin,
    submission_client: &State<SubmissionClient>,
) -> Result<Template, status::Custom<String>> {
    let submission_client = submission_client.inner().clone();
    match submission_client
        .get_submission_list(tonic::Request::new(
            submission::GetSubmissionListRequest::default(),
        ))
        .await
    {
        Ok(response) => {
            let submissions = TemplateSubmissions {
                submission_list: response
                    .into_inner()
                    .list
                    .iter()
                    .map(|q| TemplateSubmissionsItem {
                        submission_id: q.submission_id,
                        problem_id: q.submission_id,
                        user: q.user.clone(),
                        state: format!("{:?}", q.state), // TODO: convert to enum
                        time: utils::render_protos_timestamp(q.timestamp.clone(), "%F %X"),
                    })
                    .collect(),
            };
            Ok(Template::render("submissions", submissions))
        }
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateContest {
    name: String,
    description: String,
    start_time: String,
    end_time: String,
}
#[get("/contest")]
async fn contest_template(
    _admin: Admin,
    contest_client: &State<ContestClient>,
) -> Result<Template, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    match contest_client
        .get_contest_metadata(tonic::Request::new(
            contest::GetContestMetadataRequest::default(),
        ))
        .await
    {
        Ok(response) => {
            let res = response.into_inner().metadata;
            let contest = TemplateContest {
                name: res.name,
                description: res.description,
                start_time: match res.start_time {
                    Some(t) => utils::render_protos_timestamp(t, "%FT%T"),
                    None => utils::render_protos_timestamp((SystemTime::now()+std::time::Duration::from_secs(86400)).into(),"%FT%T"),
                },
                end_time: match res.end_time {
                    Some(t) => utils::render_protos_timestamp(t, "%FT%T"),
                    None => utils::render_protos_timestamp((SystemTime::now()+std::time::Duration::from_secs(93600)).into(),"%FT%T"),
                },
            };
            Ok(Template::render("contest", contest))
        }
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

// static

#[get("/")]
async fn root_logged(_admin: Admin) -> Redirect {
    Redirect::to("/home")
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
        .mount(
            "/",
            routes![
                root,
                root_logged,
                statics,
                statics_redirect,
                questions_template,
                submissions_template,
                submission_details_template,
                contest_template,
                update_contest_form,
                reply_form,
                set_user_form,
                login_form
            ],
        )
        .attach(Template::fairing())
}
