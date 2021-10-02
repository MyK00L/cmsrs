use super::auth::*;
use super::clients::*;
use protos::service::submission;
use rocket::data::Capped;
use rocket::form::{Form, Strict};
use rocket::fs::TempFile;
use rocket::http::Status;
use rocket::response::{status, Redirect};
use rocket::serde::Serialize;
use rocket::*;
use rocket_dyn_templates::Template;
use std::io::Read;
use std::str::FromStr;
use std::string::ToString;
use strum::IntoEnumIterator;

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
    languages: Vec<String>,
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
    let languages = protos::common::ProgrammingLanguage::iter()
        .map(|l| l.to_string())
        .collect();
    Ok(Template::render(
        "problems",
        ProblemsTemplate {
            contest,
            running_contest,
            problem,
            languages,
            submissions,
        },
    ))
}

#[derive(FromForm)]
pub struct SubmitForm<'v> {
    problem_id: u64,
    language: String,
    file: Capped<TempFile<'v>>,
}
#[post("/api/submit", data = "<submission>")]
pub async fn submit(
    user: User,
    _running_contest: RunningContest,
    submission: Form<Strict<SubmitForm<'_>>>,
    submission_client: &State<SubmissionClient>,
) -> Result<Redirect, status::Custom<()>> {
    if !submission.file.is_complete() {
        return Err(status::Custom(Status::PayloadTooLarge, ()));
    }
    let lang = match protos::common::ProgrammingLanguage::from_str(submission.language.as_str()) {
        Ok(lang) => lang,
        Err(_) => {
            return Err(status::Custom(Status::InternalServerError, ()));
        }
    };
    let mut raw = Vec::<u8>::new();
    let path = match submission.file.path() {
        Some(path) => path,
        None => {
            return Err(status::Custom(Status::InternalServerError, ()));
        }
    };
    let mut file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => {
            return Err(status::Custom(Status::InternalServerError, ()));
        }
    };
    if file.read_to_end(&mut raw).is_err() {
        return Err(status::Custom(Status::InternalServerError, ()));
    }
    let req = submission::EvaluateSubmissionRequest {
        sub: protos::evaluation::Submission {
            user: user.0,
            problem_id: submission.problem_id,
            source: protos::common::Source {
                lang: lang as i32,
                code: raw,
            },
        },
    };
    let mut submission_client = submission_client.inner().clone();
    match submission_client
        .evaluate_submission(tonic::Request::new(req))
        .await
    {
        Ok(_) => Ok(Redirect::to(uri!(problems(submission.problem_id)))),
        Err(_) => Err(status::Custom(Status::InternalServerError, ())),
    }
}
