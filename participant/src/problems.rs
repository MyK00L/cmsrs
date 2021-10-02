use super::auth::User;
use super::clients::*;
use super::common::*;
use protos::service::contest;
use protos::service::submission;

use rocket::http::Status;
use rocket::response::status;
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
    contest: ContestMetadata,
    problem: Problem,
    // score: f64,
    submissions: Vec<SubmissionTemplate>,
}

#[get("/problem/<id>")]
pub async fn problems(
    user: User,
    id: u64,
    contest_client: &State<ContestClient>,
    submission_client: &State<SubmissionClient>,
) -> Result<Template, status::Custom<()>> {
    let mut contest_client = contest_client.inner().clone();
    let mut submission_client = submission_client.inner().clone();
    let (metadata, submissions) = futures::join!(
        contest_client.get_contest_metadata(tonic::Request::new(
            contest::GetContestMetadataRequest::default()
        )),
        submission_client.get_submission_list(tonic::Request::new(
            submission::GetSubmissionListRequest {
                limit: None,
                user: Some(user.0),
                problem_id: Some(id),
            }
        ))
    );
    let contest: ContestMetadata = match metadata {
        Ok(response) => response.into_inner().metadata,
        Err(_) => {
            return Err(status::Custom(Status::InternalServerError, ()));
        }
    }
    .into();
    let problem = match contest.problems.iter().find(|x| x.id == id) {
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
            problem,
            submissions,
        },
    ))
}
