use super::*;
use futures::future;

#[get("/users")]
pub async fn users_template(_admin: Admin) -> Result<Template, status::Custom<String>> {
    Ok(Template::render(
        "users",
        std::collections::HashMap::<String, String>::new(),
    ))
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Resources {
    nanos: u64,
    bytes: u64,
}
impl From<protos::common::Resources> for Resources {
    fn from(r: protos::common::Resources) -> Self {
        Self {
            nanos: r.time.nanos as u64 + r.time.secs * 1000000000,
            bytes: r.memory_bytes,
        }
    }
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TestcaseResult {
    outcome: String,
    score: String,
    resources: Resources,
}
impl From<protos::evaluation::TestcaseResult> for TestcaseResult {
    fn from(tr: protos::evaluation::TestcaseResult) -> Self {
        Self {
            outcome: format!(
                "{:?}",
                protos::evaluation::testcase_result::Outcome::from_i32(tr.outcome).unwrap()
            ),
            score: format!("{:?}", tr.score),
            resources: tr.used_resources.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct SubtaskResult {
    score: String,
    testcase_results: Vec<TestcaseResult>,
}
impl From<protos::evaluation::SubtaskResult> for SubtaskResult {
    fn from(sr: protos::evaluation::SubtaskResult) -> Self {
        Self {
            score: format!("{:?}", sr.score),
            testcase_results: sr
                .testcase_results
                .into_iter()
                .map(TestcaseResult::from)
                .collect(),
        }
    }
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct CompilationResult {
    outcome: String,
    resources: Resources,
    error: String,
}
impl From<protos::evaluation::CompilationResult> for CompilationResult {
    fn from(cr: protos::evaluation::CompilationResult) -> Self {
        Self {
            outcome: format!(
                "{:?}",
                protos::evaluation::compilation_result::Outcome::from_i32(cr.outcome).unwrap()
            ),
            resources: cr.used_resources.into(),
            error: cr.error_message.unwrap_or_default(),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct EvaluationResult {
    compilation: CompilationResult,
    score: String,
    subtask_results: Vec<SubtaskResult>,
}
impl From<protos::evaluation::EvaluationResult> for EvaluationResult {
    fn from(er: protos::evaluation::EvaluationResult) -> Self {
        Self {
            compilation: er.compilation_result.into(),
            score: format!("{:?}", er.score),
            subtask_results: er
                .subtask_results
                .into_iter()
                .map(SubtaskResult::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct SubmissionDetails {
    state: String,
    user: String,
    problem_id: u64,
    lang: String,
    code: String,
    evaluation: Option<EvaluationResult>,
}
impl From<submission::GetSubmissionDetailsResponse> for SubmissionDetails {
    fn from(res: submission::GetSubmissionDetailsResponse) -> Self {
        Self {
            state: format!(
                "{:?}",
                submission::SubmissionState::from_i32(res.state).unwrap()
            ),
            user: res.sub.user.to_string(),
            problem_id: res.sub.problem_id,
            lang: format!(
                "{:?}",
                protos::common::ProgrammingLanguage::from_i32(res.sub.source.lang).unwrap()
            ),
            code: String::from_utf8(res.sub.source.code.clone())
                .unwrap_or(format!("{:?}", res.sub.source.code)),
            evaluation: res.res.map(EvaluationResult::from),
        }
    }
}

#[get("/submission/<id>")]
pub async fn submission_details_template(
    _admin: Admin,
    submission_client: &State<SubmissionClient>,
    id: u64,
) -> Result<Template, status::Custom<String>> {
    let submission_client = submission_client.inner().clone();
    match submission_client
        .get_submission_details(tonic::Request::new(
            submission::GetSubmissionDetailsRequest { submission_id: id },
        ))
        .await
    {
        Ok(response) => {
            let res = response.into_inner();
            let submission_details = SubmissionDetails::from(res);
            Ok(Template::render("submission_details", submission_details))
        }
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Question {
    id: u64,
    problem_id: Option<u64>,
    time: String,
    user: String,
    subject: String,
    text: String,
}
impl From<contest::Message> for Question {
    fn from(q: contest::Message) -> Self {
        Self {
            id: q.id,
            problem_id: q.problem_id,
            time: utils::render_protos_timestamp(q.sent_at.clone(), "%F %X"),
            user: q.from.clone().unwrap_or_default(),
            subject: q.subject.clone(),
            text: q.text,
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct QuestionList {
    questions: Vec<Question>,
}
impl From<contest::GetQuestionListResponse> for QuestionList {
    fn from(ql: contest::GetQuestionListResponse) -> Self {
        Self {
            questions: ql.questions.into_iter().map(Question::from).collect(),
        }
    }
}

#[get("/questions")]
pub async fn questions_template(
    _admin: Admin,
    contest_client: &State<ContestClient>,
) -> Result<Template, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    match contest_client
        .get_question_list(tonic::Request::new(
            contest::GetQuestionListRequest::default(),
        ))
        .await
    {
        Ok(response) => {
            let res = response.into_inner();
            let question_list = QuestionList::from(res);
            Ok(Template::render("questions", question_list))
        }
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubmissionsItem {
    submission_id: u64,
    problem_id: u64,
    user: String,
    state: String,
    time: String,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubmissions {
    submission_list: Vec<TemplateSubmissionsItem>,
}
#[get("/submissions")]
pub async fn submissions_template(
    _admin: Admin,
    submission_client: &State<SubmissionClient>,
) -> Result<Template, status::Custom<String>> {
    let submission_client = submission_client.inner().clone();
    match submission_client
        .get_submission_list(tonic::Request::new(
            submission::GetSubmissionListRequest::default(),
        ))
        .await
    {
        Ok(response) => {
            let submissions = TemplateSubmissions {
                submission_list: response
                    .into_inner()
                    .list
                    .iter()
                    .map(|q| TemplateSubmissionsItem {
                        submission_id: q.submission_id,
                        problem_id: q.submission_id,
                        user: q.user.clone(),
                        state: format!(
                            "{:?}",
                            submission::SubmissionState::from_i32(q.state).unwrap()
                        ),
                        time: utils::render_protos_timestamp(q.timestamp.clone(), "%F %X"),
                    })
                    .collect(),
            };
            Ok(Template::render("submissions", submissions))
        }
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct SubtaskScoring {
    method: String, // MIN | SUM
    max_score: f64,
}
impl From<protos::scoring::Subtask> for SubtaskScoring {
    fn from(s: protos::scoring::Subtask) -> Self {
        Self {
            method: format!(
                "{:?}",
                protos::scoring::subtask::Method::from_i32(s.method).unwrap()
            ),
            max_score: s.max_score,
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Subtask {
    id: u64,
    scoring: SubtaskScoring,
    testcases: Vec<u64>,
}
impl From<evaluation::Subtask> for Subtask {
    fn from(s: evaluation::Subtask) -> Self {
        Self {
            id: s.id,
            scoring: s.scoring.into(),
            testcases: s.testcases_id,
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct ProblemScoring {
    method: String, // SUM_MAX | MAX_SUM
}
impl From<protos::scoring::Problem> for ProblemScoring {
    fn from(p: protos::scoring::Problem) -> Self {
        Self {
            method: format!(
                "{:?}",
                protos::scoring::problem::Method::from_i32(p.method).unwrap()
            ),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Problem {
    id: u64,
    scoring: ProblemScoring,
    problem_type: String,
    execution_limits: Resources,
    compilation_limits: Resources,
    subtasks: Vec<Subtask>,
    name: String,
    longname: String,
}
impl Problem {
    fn new(e: evaluation::Problem, u: contest::Problem) -> Self {
        Self {
            id: e.id,
            scoring: e.scoring.into(),
            problem_type: String::from(""), //format!("{:?}", p.aa),
            execution_limits: e.execution_limits.into(),
            compilation_limits: e.compilation_limits.into(),
            subtasks: e.subtasks.into_iter().map(Subtask::from).collect(),
            name: u.name.clone(),
            longname: u.long_name.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct UserScoringMethod {
    aggregation: String, // Sum | Max
    score_weight: f64,
    wrong_submission_count_weight: f64,
    time_secs_weight: f64,
}
impl From<protos::scoring::user::Method> for UserScoringMethod {
    fn from(us: protos::scoring::user::Method) -> Self {
        Self {
            aggregation: us.aggregation_method.to_string(),
            score_weight: us.score_weight,
            wrong_submission_count_weight: us.wrong_submission_count_weight,
            time_secs_weight: us.time_secs_weight,
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct UserScoring {
    main: UserScoringMethod,
    tiebreakers: Vec<UserScoringMethod>,
}
impl From<protos::scoring::User> for UserScoring {
    fn from(us: protos::scoring::User) -> Self {
        Self {
            main: us.main.into(),
            tiebreakers: us
                .tiebreakers
                .into_iter()
                .map(UserScoringMethod::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct ContestTemplate {
    name: String,
    description: String,
    start_time: String,
    end_time: String,
    problems: Vec<Problem>,
    user_scoring: UserScoring,
}
impl ContestTemplate {
    fn new(
        evaluation_contest: evaluation::GetContestResponse,
        user_contest: contest::GetContestMetadataResponse,
        contest_problems: Vec<contest::Problem>,
    ) -> Self {
        Self {
            name: user_contest.metadata.name,
            description: user_contest.metadata.description,
            start_time: match user_contest.metadata.start_time {
                Some(t) => utils::render_protos_timestamp(t, "%FT%T"),
                None => utils::render_protos_timestamp(
                    (SystemTime::now() + std::time::Duration::from_secs(86400)).into(),
                    "%FT%T",
                ),
            },
            end_time: match user_contest.metadata.end_time {
                Some(t) => utils::render_protos_timestamp(t, "%FT%T"),
                None => utils::render_protos_timestamp(
                    (SystemTime::now() + std::time::Duration::from_secs(93600)).into(),
                    "%FT%T",
                ),
            },
            problems: evaluation_contest.info.problems.into_iter().zip(contest_problems.into_iter()).map(|x| Problem::new(x.0,x.1)).collect(),
            user_scoring: evaluation_contest.info.user_scoring_method.into(),
        }
    }
}

#[get("/contest")]
pub async fn contest_template(
    _admin: Admin,
    contest_client: &State<ContestClient>,
    evaluation_client: &State<EvaluationClient>,
) -> Result<Template, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    let evaluation_client = evaluation_client.inner().clone();
    let (user_contest_response, evaluation_contest_response) = future::join(
        contest_client.get_contest_metadata(tonic::Request::new(
            contest::GetContestMetadataRequest::default(),
        )),
        evaluation_client
            .get_contest(tonic::Request::new(evaluation::GetContestRequest::default())),
    )
    .await;
    let user_contest = match user_contest_response {
        Ok(response) => response.into_inner(),
        Err(err) => {
            return Err(status::Custom(
                Status::InternalServerError,
                format!("Error in rpc request:\n{:?}", err),
            ));
        }
    };
    let evaluation_contest = match evaluation_contest_response {
        Ok(response) => response.into_inner(),
        Err(err) => {
            return Err(status::Custom(
                Status::InternalServerError,
                format!("Error in rpc request:\n{:?}", err),
            ));
        }
    };
    let user_problem_requests: Vec<
        core::pin::Pin<
            Box<
                dyn futures::Future<
                        Output = Result<
                            tonic::Response<contest::GetProblemResponse>,
                            tonic::Status,
                        >,
                    > + std::marker::Send,
            >,
        >,
    > = evaluation_contest
        .info
        .problems
        .clone()
        .into_iter()
        .map(|x| {
            contest_client.get_problem(tonic::Request::new(contest::GetProblemRequest {
                problem_id: x.id,
            }))
        })
        .collect();
    let user_problem_responses: Vec<contest::Problem> = future::join_all(user_problem_requests)
        .await
        .into_iter()
        .map(
            |x: Result<tonic::Response<contest::GetProblemResponse>, tonic::Status>| {
                x.unwrap().into_inner().info // TODO: remove unwrap
            },
        )
        .collect();

    let ct = ContestTemplate::new(evaluation_contest, user_contest, user_problem_responses);
    Ok(Template::render("contest", ct))
}
