use rocket::form::{Form, Strict};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::IntoOutcome;
use rocket::request::FromRequest;
use rocket::response::Redirect;
//use rocket::response::content::Html;
use rocket::*;
use protos::service::contest::contest_server::Contest;
//use rocket::http::RawStr;
use rocket_dyn_templates::Template;
use rocket::serde::Serialize;
use std::path::{PathBuf, Path};
use rocket::fs::{NamedFile,relative};

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

// templates

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateContext {
    hello: String,
}
#[get("/tem")]
async fn template_try() -> Template {
    Template::render("home",TemplateContext{hello:String::from("owo//\\")})
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateQuestion {
    id: u64,
    subject: String,
    text: String
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateQuestions {
    questions: Vec<TemplateQuestion>
}
#[get("/questions")]
async fn questions(_admin: Admin, contest_client: &State<ContestClient>) -> Option<Template> {
    let contest_client = contest_client.inner().clone();
    match contest_client.get_question_list(tonic::Request::new(protos::service::contest::GetQuestionListRequest::default())).await.ok() {
        Some(response) => {
            let questions = TemplateQuestions{questions:response.into_inner().questions.iter().map(|q|{
                TemplateQuestion{
                    id: 42,
                    subject: q.subject.clone(),
                    text: q.text.clone(),
                }
            }).collect()};
            Some(Template::render("questions",
                questions
            ))
        }
        None => None
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

#[rocket::get("/<path..>", rank=7)]
async fn statics(_admin: Admin, path: PathBuf) -> Option<NamedFile> {
    let mut path = Path::new(relative!("static")).join(path);
    if path.is_dir() {
        path.push("index.html");
    }
    NamedFile::open(path).await.ok()
}
#[rocket::get("/<_path..>", rank=8)]
async fn statics_redirect(_path: PathBuf) -> Redirect {
    Redirect::to(uri!(root))
}

// main

#[launch]
fn rocket() -> _ {
    let mut contest_client = protos::service::contest::MockContest::default();
    contest_client.get_question_list_set(protos::service::contest::GetQuestionListResponse{
        questions: vec![
            protos::user::Message{subject:String::from("Problem A"),text:String::from("oh hi"),user:Some(String::from("me")),..Default::default()}
        ]
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
                template_try
            ],
        ).attach(Template::fairing())
}
