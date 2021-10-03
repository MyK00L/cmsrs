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
use std::convert::TryFrom;
use std::convert::TryInto;

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


// submission details

#[derive(Serialize, FromForm, Debug, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Resources {
    nanos: u64,
    bytes: u64,
}
impl TryFrom<protos::common::Resources> for Resources {
    type Error = (); // !
    fn try_from(r: protos::common::Resources) -> Result<Self,Self::Error> {
        Ok(Self {
            nanos: r.time.nanos as u64 + r.time.secs * 1000000000,
            bytes: r.memory_bytes,
        })
    }
}
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct CompilationResult {
    outcome: String,
    resources: Resources,
}
impl TryFrom<protos::evaluation::CompilationResult> for CompilationResult {
    type Error = ();
    fn try_from(cr: protos::evaluation::CompilationResult) -> Result<Self,Self::Error> {
        Ok(Self {
            outcome: protos::evaluation::compilation_result::Outcome::from_i32(cr.outcome).ok_or(())?.to_string(),
            resources: cr.used_resources.try_into()?,
        })
    }
}
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct TestcaseResult {
    outcome: String,
    score: String,
    resources: Resources,
}
impl TryFrom<protos::evaluation::TestcaseResult> for TestcaseResult {
    type Error = ();
    fn try_from(tr: protos::evaluation::TestcaseResult) -> Result<Self,Self::Error> {
        Ok(Self {
            outcome: protos::evaluation::testcase_result::Outcome::from_i32(tr.outcome).ok_or(())?.to_string(),
            score: tr.score.score.to_string(),
            resources: tr.used_resources.try_into()?,
        })
    }
}
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct SubtaskResult {
    score: String,
    testcase_results: Vec<TestcaseResult>,
}
impl TryFrom<protos::evaluation::SubtaskResult> for SubtaskResult {
    type Error = ();
    fn try_from(sr: protos::evaluation::SubtaskResult) -> Result<Self,Self::Error> {
        Ok(Self {
            score: sr.score.score.to_string(),
            testcase_results: sr
                .testcase_results
                .into_iter()
                .map(|x| TestcaseResult::try_from(x)?)
                .collect(),
        })
    }
}
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct EvaluationResult {
    compilation: CompilationResult,
    score: String,
    subtask_results: Vec<SubtaskResult>,
}
impl TryFrom<protos::evaluation::EvaluationResult> for EvaluationResult {
    type Error = ();
    fn try_from(er: protos::evaluation::EvaluationResult) -> Result<Self,Self::Error> {
        Ok(Self {
            compilation: er.compilation_result.try_into()?,
            score: er.score.score.to_string(),
            subtask_results: er
                .subtask_results
                .into_iter()
                .map(|x| SubtaskResult::try_from(x)?)
                .collect(),
        })
    }
}
#[derive(Serialize, Debug, Clone)]
#[serde(crate = "rocket::serde")]
struct SubmissionDetails {
    state: String,
    problem_id: u64,
    lang: String,
    code: String,
    evaluation: Option<EvaluationResult>,
}
impl TryFrom<submission::GetSubmissionDetailsResponse> for SubmissionDetails {
    type Error = ();
    fn try_from(res: submission::GetSubmissionDetailsResponse) -> Result<Self,Self::Error> {
        Ok(Self {
            state: submission::SubmissionState::from_i32(res.state).ok_or(())?.to_string(),
            problem_id: res.sub.problem_id,
            lang: protos::common::ProgrammingLanguage::from_i32(res.sub.source.lang).ok_or(())?.to_string(),
            code: String::from_utf8(res.sub.source.code.clone()).map_err(|_|())?,
            evaluation: res.res.map(|x| EvaluationResult::try_from(x)?),
        })
    }
}

#[get("/submission/<id>")]
pub async fn submission_details_template(
    user: User,
    _running_contest: RunningContest,
    submission_client: &State<SubmissionClient>,
    id: u64,
) -> Result<Template, status::Custom<()>> {
    let mut submission_client = submission_client.inner().clone();
    match submission_client
        .get_submission_details(tonic::Request::new(
            submission::GetSubmissionDetailsRequest { submission_id: id },
        ))
        .await
    {
        Ok(response) => {
            let res = response.into_inner();
            let submission_details = SubmissionDetails::from(res.try_into().unwrap());
            Ok(Template::render("submission_details", submission_details))
        }
        Err(_) => Err(status::Custom(Status::InternalServerError,())),
    }
}

