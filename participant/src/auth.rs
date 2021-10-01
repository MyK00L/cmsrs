use rocket::form::{Form, Strict};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::IntoOutcome;
use rocket::request::FromRequest;
use rocket::response::{status, Redirect};
use rocket::*;
use rocket_dyn_templates::Template;

pub struct User(String);
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
pub async fn root() -> Result<Template, status::Custom<String>> {
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
pub async fn login(cookies: &CookieJar<'_>, login: Form<Strict<Login>>) -> Redirect {
    // TODO check password
    #[allow(clippy::branches_sharing_code)] // clippy be weird
    if login.pass == *"hi" {
        cookies.add_private(Cookie::new("user", login.name.clone()));
        Redirect::to(uri!(root_logged))
    } else {
        Redirect::to(uri!(root))
    }
}

#[post("/api/logout")]
pub async fn logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::named("user"));
    Redirect::to(uri!(root))
}
