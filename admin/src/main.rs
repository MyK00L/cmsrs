use rocket::response::Redirect;
use rocket::*;
use rocket::request::FromRequest;
use rocket::outcome::IntoOutcome;
use rocket::http::{CookieJar, Cookie};

struct Admin;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Admin {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Admin, ()> {
        request.cookies()
            .get_private("admin")
            .and_then(|_cookie| Some(Admin{}))
            .or_forward(())
    }
}

#[get("/")]
fn login(cookies: &CookieJar<'_>) -> &'static str {
    cookies.add_private(Cookie::new("admin","admin"));
    "Hello, this is login"
}

#[get("/admin")]
fn admin_panel(_admin: Admin) -> &'static str {
    "Hello, administrator. This is the admin panel!"
}

#[get("/admin", rank = 2)]
fn admin_panel_redirect() -> Redirect {
    Redirect::to(uri!(login))
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![login,admin_panel,admin_panel_redirect])
}

