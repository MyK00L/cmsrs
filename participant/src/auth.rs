use super::clients::*;
use protos::service::contest;
use rocket::form::{Form, Strict};
use rocket::http::Status;
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::IntoOutcome;
use rocket::request::FromRequest;
use rocket::response::{status, Redirect};
use rocket::*;
use rocket_dyn_templates::Template;

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

#[get("/")]
pub async fn root_logged(_user: User) -> Redirect {
    // TODO redirect to main page
    Redirect::to(uri!(super::questions::questions))
}
#[get("/", rank = 2)]
pub async fn root() -> Result<Template, status::Custom<()>> {
    Ok(Template::render(
        "login",
        std::collections::HashMap::<String, String>::new(),
    ))
}

#[rocket::get("/<_path..>", rank = 8)]
pub async fn not_logged_redirect(_path: std::path::PathBuf) -> Redirect {
    Redirect::to(uri!(root))
}

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
