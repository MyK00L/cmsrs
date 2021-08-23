use rocket::form::{Form, Strict};
use rocket::http::{Cookie, CookieJar};
use rocket::outcome::IntoOutcome;
use rocket::request::FromRequest;
use rocket::response::Redirect;
use rocket::response::content::Html;
use rocket::*;

const PASS: &str = "1234";

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
        Redirect::to(uri!(home))
    } else {
        Redirect::to(uri!(root))
    }
}
use protos::service::contest::contest_server::Contest;
#[get("/prova")]
async fn prova(contest_client_a: &State<protos::service::contest::MockContest>) -> String {
    let contest_client = contest_client_a.inner().clone();
    let res = contest_client.auth_user(tonic::Request::new(protos::service::contest::AuthUserRequest{passw:String::from("Hi"),name:String::from("Hi")})).await;
    format!("hi {:?}", res)
}

// pages

#[get("/")]
async fn root_logged(_admin: Admin) -> Redirect {
    Redirect::to(uri!(home))
}
#[get("/", rank = 2)]
async fn root() -> Html<String> {
    Html(format!("{}", include_str!("../www/login.html")))
}

#[get("/home")]
async fn home(_admin: Admin) -> Html<String> {
    Html(format!("{}", include_str!("../www/home.html")))
}
#[get("/home", rank = 2)]
async fn home_redirect() -> Redirect {
    Redirect::to(uri!(root))
}

#[launch]
fn rocket() -> _ {
    let contest_client = protos::service::contest::MockContest::default();
    rocket::build()
        .manage(contest_client)
        .mount(
            "/",
            routes![
                prova,
                root,
                root_logged,
                home,
                home_redirect,
                login_form
            ],
        )
}
