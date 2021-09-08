use super::*;

// templates

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
/*#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateResources {
    nanos: u64,
    bytes: u64,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubtaskScoring {
    method: String, // MIN | SUM
    max_score: f64,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateSubtask {
    id: u64,
    scoring: TemplateSubtaskScoring,
    testcases: Vec<u64>,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateProblemScoring {
    method: String, // SUM_MAX | MAX_SUM
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateProblem {
    id: u64,
    scoring: TemplateProblemScoring,
    problem_type: String,
    execution_limits: TemplateResources,
    compilation_limits: TemplateResources,
    subtasks: Vec<TemplateSubtask>,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateUserScoringMethod {
    aggregation: String, // SUM | MAX
    score_weight: f64,
    wrong_submision_count_weight: f64,
    time_secs_weight: f64,
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct TemplateContest {
    name: String,
    description: String,
    start_time: String,
    end_time: String,
    user_scoring: Vec<TemplateUserScoringMethod>,
    problems: Vec<TemplateProblem>,
}*/
#[get("/contest")]
pub async fn contest_template(
    _admin: Admin,
    contest_client: &State<ContestClient>,
) -> Result<Template, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    match contest_client
        .get_contest_metadata(tonic::Request::new(
            contest::GetContestMetadataRequest::default(),
        ))
        .await
    {
        Ok(response) => {
            // TODO: timezones
            let res = response.into_inner().metadata;
            /*let contest = TemplateContest {
                name: res.name,
                description: res.description,
                start_time: match res.start_time {
                    Some(t) => utils::render_protos_timestamp(t, "%FT%T"),
                    None => utils::render_protos_timestamp(
                        (SystemTime::now() + std::time::Duration::from_secs(86400)).into(),
                        "%FT%T",
                    ),
                },
                end_time: match res.end_time {
                    Some(t) => utils::render_protos_timestamp(t, "%FT%T"),
                    None => utils::render_protos_timestamp(
                        (SystemTime::now() + std::time::Duration::from_secs(93600)).into(),
                        "%FT%T",
                    ),
                },
            };*/
            let contest = res;
            Ok(Template::render("contest", contest))
        }
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}