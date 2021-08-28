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
use std::convert::TryInto;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use utils::gen_uuid;
//use std::convert::TryFrom;

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
    compilation_secs: u64,
    compilation_bytes: u64,
    compilation_error: String,
    overall_score: String,
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
                        compilation_outcome: res.compilation_result.outcome.to_string(),
                        compilation_secs: res
                            .compilation_result
                            .used_resources
                            .time
                            .seconds
                            .try_into()
                            .unwrap_or(0),
                        compilation_bytes: res.compilation_result.used_resources.memory_bytes,
                        compilation_error: res
                            .compilation_result
                            .error_message
                            .unwrap_or(String::from("")),
                        overall_score: format!("{:?}", res.overall_score.unwrap()),
                        subtask_results: res
                            .subtask_results
                            .iter()
                            .map(|sr| TemplateSubtaskResult {
                                n: 0,
                                score: format!("{:?}", sr.subtask_score.as_ref().unwrap()),
                                testcase_results: sr
                                    .testcase_results
                                    .iter()
                                    .map(|tr| TemplateTestcaseResult {
                                        verdict: format!("{:?}", tr.verdict.as_ref().unwrap()),
                                        outcome: tr.outcome.to_string(),
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
                        time: utils::render_prost_timestamp(q.sent_at.clone(), "%F %X"),
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
                        time: utils::render_prost_timestamp(q.timestamp.clone(), "%F %X"),
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
            overall_score: Some(
                submission::get_submission_list_response::item::OverallScore::DoubleScore(42.0),
            ),
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
                ..Default::default()
            },
        },
        ..Default::default()
    });
    contest_client.add_message_set(contest::AddMessageResponse::default());
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
                login_form,
                reply_form,
                set_user_form,
            ],
        )
        .attach(Template::fairing())
}
