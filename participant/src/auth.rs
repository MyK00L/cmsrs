use super::clients::*;
use protos::service::contest;
use rocket::form::{Form, Strict};
use rocket::http::Status;
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::IntoOutcome;
use rocket::request::FromRequest;
use rocket::response::{status, Redirect};
use rocket::*;
use rocket::fs::{relative, NamedFile};
use std::path::{Path, PathBuf};

pub struct User(pub String);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<User, ()> {
        request
            .cookies()
            .get_private("user")
            .map(|cookie| User(cookie.value().into()))
            .or_forward(())
    }
}

// login page, redirects if already logged
#[get("/")]
pub async fn root_logged(_user: User) -> Redirect {
    // TODO redirect to main page
    Redirect::to(uri!(super::questions::questions))
}
#[get("/", rank = 2)]
pub async fn root() -> Option<NamedFile> {
    let path = Path::new(relative!("static/login/index.html"));
    NamedFile::open(path).await.ok()
}

// can access static files other than login only if logged
#[get("/<path..>", rank = 7)]
pub async fn statics(_user: User, path: PathBuf) -> Option<NamedFile> {
    let mut path = Path::new(relative!("static")).join(path);
    if path.is_dir() {
        path.push("index.html");
    }
    NamedFile::open(path).await.ok()
}
#[get("/<_path..>", rank = 8)]
pub async fn not_logged_redirect(_path: PathBuf) -> Redirect {
    Redirect::to(uri!(root))
}

// login and logout apis
#[derive(FromForm)]
pub struct Login {
    name: String,
    pass: String,
}
#[post("/api/login", data = "<login>")]
pub async fn login(
    cookies: &CookieJar<'_>,
    contest_client: &State<ContestClient>,
    login: Form<Strict<Login>>,
) -> Result<Redirect, status::Custom<()>> {
    let mut contest_client = contest_client.inner().clone();
    match contest_client
        .auth_user(tonic::Request::new(contest::AuthUserRequest {
            username: login.name.clone(),
            password: login.pass.clone(),
        }))
        .await
    {
        Ok(response) => match response.into_inner().response.unwrap() {
            contest::auth_user_response::Response::Success(_) => {
                cookies.add_private(Cookie::new("user", login.name.clone()));
                Ok(Redirect::to(uri!(root_logged)))
            }
            _ => Ok(Redirect::to(uri!(root))),
        },
        Err(_) => Err(status::Custom(Status::InternalServerError, ())),
    }
}

#[get("/api/logout")]
pub async fn logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::named("user"));
    Redirect::to(uri!(root))
}
