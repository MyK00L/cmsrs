use super::clients::*;
use protos::service::contest;
use rocket::form::{Form, Strict};
use rocket::fs::{relative, NamedFile};
use rocket::http::Status;
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::{try_outcome,IntoOutcome};
use rocket::request::{FromRequest, Outcome};
use rocket::response::{status, Redirect};
use rocket::serde::Serialize;
use rocket::*;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
        Ok(response) => match response
            .into_inner()
            .response
            .ok_or(status::Custom(Status::InternalServerError, ()))?
        {
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

// contest metadata request utilities

struct ContestMetadataWrapper(contest::ContestMetadata);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r ContestMetadataWrapper {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // This closure will execute at most once per request, regardless of
        // the number of times the `RunningContest` guard is executed.
        let result = request
            .local_cache_async(async {
                let mut contest_client = request
                    .guard::<&State<ContestClient>>()
                    .await
                    .succeeded()?
                    .inner()
                    .clone();
                let metadata = contest_client
                    .get_contest_metadata(tonic::Request::new(
                        contest::GetContestMetadataRequest::default(),
                    ))
                    .await
                    .ok()?
                    .into_inner()
                    .metadata;
                Some(ContestMetadataWrapper(metadata))
            })
            .await;
        result.as_ref().or_forward(())
    }
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Problem {
    pub id: u64,
    pub name: String,
}
impl From<contest::Problem> for Problem {
    fn from(p: contest::Problem) -> Self {
        Self {
            id: p.id,
            name: p.name,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct RunningContest {
    pub problems: Vec<Problem>,
}
#[rocket::async_trait]
impl<'r> FromRequest<'r> for RunningContest {
    type Error = std::convert::Infallible;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let now = SystemTime::now();
        let metadata = &try_outcome!(request.guard::<&ContestMetadataWrapper>().await).0;
        let is_running = match (metadata.start_time.as_ref(), metadata.end_time.as_ref()) {
            (Some(start_time), Some(end_time)) => {
                now >= SystemTime::from(start_time.clone()) && now < SystemTime::from(end_time.clone())
            }
            _ => false,
        };
        if is_running {
            Outcome::Success(RunningContest {
                problems: vec![Problem {
                    id: 42,
                    name: String::from("problem ei"),
                }],
            })
        } else {
            Outcome::Forward(())
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct ContestData {
    pub name: String,
    pub start_time: Option<String>, // millis from unix epoch
    pub end_time: Option<String>,
}
impl From<&contest::ContestMetadata> for ContestData {
    fn from(c: &contest::ContestMetadata) -> Self {
        Self {
            name: c.name.clone(),
            start_time: c
                .start_time.clone()
                .map(|t| {
                    SystemTime::from(t)
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .ok()
                })
                .flatten()
                .map(|t| t.as_millis().to_string()),
            end_time: c
                .end_time.clone()
                .map(|t| {
                    SystemTime::from(t)
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .ok()
                })
                .flatten()
                .map(|t| t.as_millis().to_string()),
        }
    }
}
#[rocket::async_trait]
impl<'r> FromRequest<'r> for ContestData {
    type Error = std::convert::Infallible;
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let metadata = &try_outcome!(request.guard::<&ContestMetadataWrapper>().await).0;
        Outcome::Success(metadata.into())
    }
}
