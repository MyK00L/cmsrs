use super::auth::User;
use super::clients::*;
use super::common::*;
use protos::service::contest;
use rocket::form::{Form, Strict};
use rocket::http::Status;
use rocket::response::{status, Redirect};
use rocket::serde::Serialize;
use rocket::*;
use rocket_dyn_templates::Template;
use std::time::SystemTime;
use utils::gen_uuid;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SubmissionTemplate {
    score: f64,
    id: u64,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ProblemsTemplate {
    contest: ContestMetadata,
    problem: Problem,
    score: f64,
    submissions: Vec<SubmissionTemplate>,
}
#[get("/problem/<id>")]
pub async fn problems(user: User, id: u64, contest_client: &State<ContestClient>, submission_client: &State<SubmissionClient>) -> Result<Template, status::Custom<()>> {
    unimplemented!();
}


