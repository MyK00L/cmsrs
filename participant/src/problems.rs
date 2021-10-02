use super::auth::*;
use super::clients::*;
use protos::service::{contest, submission};
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::response::{status, Redirect};
use rocket::serde::Serialize;
use rocket::*;
use rocket_dyn_templates::Template;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SubmissionTemplate {
    id: u64,
    score: String,
}
impl From<submission::get_submission_list_response::Item> for SubmissionTemplate {
    fn from(s: submission::get_submission_list_response::Item) -> Self {
        Self {
            id: s.submission_id,
            score: s.score.unwrap_or_default().score.to_string(),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ProblemsTemplate {
    contest: ContestData,
    running_contest: RunningContest,
    problem: Problem,
    // score: f64,
    submissions: Vec<SubmissionTemplate>,
}

#[get("/problem/<id>")]
pub async fn problems(
    user: User,
    id: u64,
    contest: ContestData,
    running_contest: RunningContest,
    submission_client: &State<SubmissionClient>,
) -> Result<Template, status::Custom<()>> {
    let mut submission_client = submission_client.inner().clone();
    let submissions = submission_client
        .get_submission_list(tonic::Request::new(submission::GetSubmissionListRequest {
            limit: None,
            user: Some(user.0),
            problem_id: Some(id),
        }))
        .await;
    let problem = match running_contest.problems.iter().find(|x| x.id == id) {
        Some(x) => x.clone(),
        None => {
            return Err(status::Custom(Status::InternalServerError, ()));
        }
    };
    let submissions: Vec<SubmissionTemplate> = match submissions {
        Ok(response) => response
            .into_inner()
            .list
            .into_iter()
            .map(SubmissionTemplate::from)
            .collect(),
        Err(_) => {
            return Err(status::Custom(Status::InternalServerError, ()));
        }
    };
    Ok(Template::render(
        "problems",
        ProblemsTemplate {
            contest,
            running_contest,
            problem,
            submissions,
        },
    ))
}

#[derive(FromForm)]
pub struct SubmitForm<'v> {
    problem_id: u64,
    language: String,
    file: TempFile<'v>,
}
#[post("/api/submit")]
pub async fn submit(
    user: User,
    _running_contest: RunningContest,
    submission_client: &State<SubmissionClient>,
) -> Result<Redirect, status::Custom<()>> {
    unimplemented!();
}
