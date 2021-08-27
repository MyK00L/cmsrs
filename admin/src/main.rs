use protos::service::{contest,submission};
use protos::service::contest::contest_server::Contest;
use protos::service::submission::submission_server::Submission;
use rocket::form::{Form, Strict};
use rocket::fs::{relative, NamedFile};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::IntoOutcome;
use rocket::request::FromRequest;
use rocket::response::Redirect;
use rocket::serde::Serialize;
use rocket::*;
use rocket_dyn_templates::Template;
use std::convert::TryFrom;
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
) -> Result<Redirect, String> {
    let contest_client = contest_client.inner().clone();
    let req = contest::SetUserRequest {
        username: user.username.clone(),
        fullname: user.fullname.clone(),
        password: user.password.clone(),
    };
    match contest_client.set_user(tonic::Request::new(req)).await {
        Ok(_) => Ok(Redirect::to("/users")),
        Err(err) => Err(format!("Error in sending request:\n{:?}", err)),
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
) -> Result<Redirect, String> {
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
        Err(err) => Err(format!("Error in sending request:\n{:?}", err)),
    }
}


// TODO: respond with response instead of option

// templates

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubmissionDetails {
    code: String,
}
#[get("/submission/<id>")]
async fn submission_details_template(_admin: Admin, submission_client: &State<SubmissionClient>, id: u64) -> Option<Template> {
    let submission_client = submission_client.inner().clone();
    match submission_client
        .get_submission_details(tonic::Request::new(
            submission::GetSubmissionDetailsRequest{submission_id:id},
        ))
        .await
        .ok()
    {
        Some(response) => {
            let res = response.into_inner();
            let submission_details = TemplateSubmissionDetails {
                code: String::from_utf8(res.sub.source.code).unwrap(),
            };
            Some(Template::render("submission_details", submission_details))
        }
        None => None,
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
async fn questions_template(_admin: Admin, contest_client: &State<ContestClient>) -> Option<Template> {
    let contest_client = contest_client.inner().clone();
    match contest_client
        .get_question_list(tonic::Request::new(
            contest::GetQuestionListRequest::default(),
        ))
        .await
        .ok()
    {
        Some(response) => {
            let questions = TemplateQuestions {
                questions: response
                    .into_inner()
                    .questions
                    .iter()
                    .map(|q| TemplateQuestion {
                        id: q.id,
                        problem_id: q.problem_id,
                        time: match SystemTime::try_from(q.sent_at.clone()) {
                            Ok(t) => match SystemTime::now().duration_since(t) {
                                Ok(elapsed) => format!("{}s ago", elapsed.as_secs()),
                                Err(_) => String::from("err"),
                            },
                            Err(_) => String::from("err"),
                        },
                        user: q.from.clone().unwrap_or_else(|| String::from("")),
                        subject: q.subject.clone(),
                        text: q.text.clone(),
                    })
                    .collect(),
            };
            Some(Template::render("questions", questions))
        }
        None => None,
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
async fn submissions_template(_admin: Admin, submission_client: &State<SubmissionClient>) -> Option<Template> {
    let submission_client = submission_client.inner().clone();
    match submission_client
        .get_submission_list(tonic::Request::new(
            submission::GetSubmissionListRequest::default(),
        ))
        .await
        .ok()
    {
        Some(response) => {
            let submissions = TemplateSubmissions {
                submission_list: response
                    .into_inner()
                    .list
                    .iter()
                    .map(|q| TemplateSubmissionsItem {
                        submission_id: q.submission_id,
                        problem_id: q.submission_id,
                        user: q.user.clone(),
                        state: format!("{:?}",q.state), // TODO: convert to enum
                        time: match SystemTime::try_from(q.timestamp.clone()) { // TODO: render times as absolute times
                            Ok(t) => match SystemTime::now().duration_since(t) {
                                Ok(elapsed) => format!("{}s ago", elapsed.as_secs()),
                                Err(_) => String::from("err"),
                            },
                            Err(_) => String::from("err"),
                        },
                    })
                    .collect(),
            };
            Some(Template::render("submissions", submissions))
        }
        None => None,
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
    submission_client.get_submission_list_set(submission::GetSubmissionListResponse{
        list: vec![
            submission::get_submission_list_response::Item{
                submission_id: 42,
                user: String::from("pippo"),
                problem_id: 2,
                state: submission::SubmissionState::Evaluated as i32,
                timestamp: SystemTime::now().into(),
                overall_score: Some(submission::get_submission_list_response::item::OverallScore::DoubleScore(42.0))
            }
        ]
    });
    submission_client.get_submission_details_set(submission::GetSubmissionDetailsResponse{
        sub:protos::evaluation::Submission{
            user: String::from("pippo"),
            problem_id: 2,
            source: protos::common::Source{
                code:"#define OII\nint main(){\n\treturn 0;\n}".as_bytes().to_vec(),
                ..Default::default()
            }
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
