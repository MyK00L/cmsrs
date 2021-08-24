use protos::service::contest::contest_server::Contest;
use rocket::form::{Form, Strict};
use rocket::fs::{relative, NamedFile};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::IntoOutcome;
use rocket::request::FromRequest;
use rocket::response::Redirect;
use rocket::serde::Serialize;
use rocket::*;
use rocket_dyn_templates::Template;
use std::path::{Path, PathBuf};

const PASS: &str = "1234";
type ContestClient = protos::service::contest::MockContest;

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

// API (forms)

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
    name: String,
    passw: String,
}
#[post("/form/add_user", data = "<user>")]
async fn add_user_form(
    _admin: Admin,
    user: Form<Strict<AddUserForm>>,
    contest_client: &State<ContestClient>,
) -> Result<(), String> {
    let contest_client = contest_client.inner().clone();
    let req = protos::service::contest::SetUserRequest {
        name: user.name.clone(),
        passw: user.passw.clone(),
    };
    match contest_client.set_user(tonic::Request::new(req)).await {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Error in sending request:\n{:?}", err)),
    }
}

#[derive(FromForm)]
struct ReplyForm {
    user: String,
    subject: String,
    text: String,
    broadcast: Option<bool>,
}
#[post("/form/reply", data = "<message>")]
async fn reply_form(
    _admin: Admin,
    message: Form<Strict<ReplyForm>>,
    contest_client: &State<ContestClient>,
) -> Result<(), String> {
    let contest_client = contest_client.inner().clone();
    let req = protos::service::contest::AddAnnouncementRequest {
        announcement: Some(protos::user::Message {
            subject: message.subject.clone(),
            problem_id: None,
            text: message.text.clone(),
            user: if Some(true) == message.broadcast {
                None
            } else {
                Some(message.user.clone())
            },
            timestamp: None,
        }),
    };
    match contest_client
        .add_announcement(tonic::Request::new(req))
        .await
    {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("Error in sending request:\n{:?}", err)),
    }
}

// templates

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateQuestion {
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
async fn questions(_admin: Admin, contest_client: &State<ContestClient>) -> Option<Template> {
    let contest_client = contest_client.inner().clone();
    match contest_client
        .get_question_list(tonic::Request::new(
            protos::service::contest::GetQuestionListRequest::default(),
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
                        user: q.user.clone().unwrap_or_else(|| String::from("")),
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
    let mut contest_client = protos::service::contest::MockContest::default();
    contest_client.get_question_list_set(protos::service::contest::GetQuestionListResponse {
        questions: vec![
            protos::user::Message {
                subject: String::from("Problem A"),
                text: String::from("oh hi"),
                user: Some(String::from("me")),
                ..Default::default()
            },
            protos::user::Message {
                subject: String::from("Problem AA"),
                text: String::from("///"),
                user: Some(String::from("a")),
                ..Default::default()
            },
            protos::user::Message {
                subject: String::from("Problem C"),
                text: String::from("uwu"),
                user: Some(String::from("b")),
                ..Default::default()
            },
        ],
    });
    rocket::build()
        .manage(contest_client)
        .mount(
            "/",
            routes![
                root,
                root_logged,
                statics,
                statics_redirect,
                questions,
                login_form,
                reply_form,
                add_user_form,
            ],
        )
        .attach(Template::fairing())
}
