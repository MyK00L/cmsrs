use rocket::form::{Form, Strict};
use rocket::fs::{relative, NamedFile};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::IntoOutcome;
use rocket::request::FromRequest;
use rocket::response::Redirect;
use rocket::*;
use std::path::{Path, PathBuf};

const PASS: &str = "1234";

struct Admin {}
#[rocket::async_trait]
impl<'r> FromRequest<'r> for Admin {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Admin, ()> {
        request
            .cookies()
            .get_private("admin")
            .and_then(|_cookie| Some(Admin {}))
            .or_forward(())
    }
}

// forms

#[derive(FromForm)]
struct LoginForm {
    pass: String,
}
#[post("/form/login", data = "<login>")]
async fn login_form(cookies: &CookieJar<'_>, login: Form<Strict<LoginForm>>) -> Redirect {
    if login.pass == PASS {
        cookies.add_private(Cookie::new("admin", "admin"));
        Redirect::to("/home.html")
    } else {
        Redirect::to(uri!(root))
    }
}

// static

#[rocket::get("/<path..>",rank=6)]
async fn static_files(_admin: Admin, path: PathBuf) -> Option<NamedFile> {
    let mut path = Path::new(relative!("static")).join(path);
    if path.is_dir() {
        path.push("index.html");
    }
    eprintln!("serving static {:?}", path);
    NamedFile::open(path).await.ok()
}
#[rocket::get("/<_path..>",rank=7)]
async fn static_files_redirect(_admin: Admin, _path: PathBuf) -> Redirect {
    Redirect::to(uri!(root))
}

#[get("/")]
async fn root_logged(_admin: Admin) -> Redirect {
    Redirect::to("/home.html")
}
#[get("/", rank = 2)]
async fn root() -> Option<NamedFile> {
    let mut path = Path::new(relative!("static/login.html"));
    NamedFile::open(path).await.ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![
                root,
                root_logged,
                static_files,
                static_files_redirect,
                login_form
            ],
        )
}
