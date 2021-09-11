use super::*;

// API (forms and stuff)

#[derive(FromForm)]
pub struct Login {
    pass: String,
}
#[post("/form/login", data = "<login>")]
pub async fn login(cookies: &CookieJar<'_>, login: Form<Strict<Login>>) -> Redirect {
    if login.pass == PASS {
        cookies.add_private(Cookie::new("admin", "admin"));
        Redirect::to(uri!(templates::submissions_template))
    } else {
        Redirect::to(uri!(root))
    }
}

#[post("/form/update_contest", data = "<contest>")]
pub async fn update_contest(
    _admin: Admin,
    contest: Form<Strict<templates::ContestTemplate>>,
    contest_client: &State<ContestClient>,
    evaluation_client: &State<EvaluationClient>,
) -> Result<Redirect, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    let evaluation_client = evaluation_client.inner().clone();
    let mut contest = contest.into_inner().into_inner();
    contest.gen_ids_if_none();

    let user_req = contest::SetContestMetadataRequest::from(contest.clone());

    let evaluation_req = evaluation::SetContestRequest::from(contest);

    let (contest_response, evaluation_response) = future::join(
        contest_client.set_contest_metadata(tonic::Request::new(user_req)),
        evaluation_client.set_contest(tonic::Request::new(evaluation_req)),
    )
    .await;
    match contest_response.ok().zip(evaluation_response.ok()) {
        Some(_) => Ok(Redirect::to("/contest")),
        None => Err(status::Custom(
            Status::InternalServerError,
            String::from("Error in sending requests :("),
        )),
    }
}

#[derive(FromForm)]
pub struct SetUser {
    username: String,
    fullname: String,
    password: String,
}
#[post("/form/set_user", data = "<user>")]
pub async fn set_user(
    _admin: Admin,
    user: Form<Strict<SetUser>>,
    contest_client: &State<ContestClient>,
) -> Result<Redirect, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    let req = contest::SetUserRequest {
        username: user.username.clone(),
        fullname: user.fullname.clone(),
        password: user.password.clone(),
    };
    match contest_client.set_user(tonic::Request::new(req)).await {
        Ok(_) => Ok(Redirect::to("/users")),
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

#[derive(FromForm)]
pub struct Reply {
    user: String,
    subject: String,
    problem_id: Option<u64>,
    text: String,
    broadcast: Option<bool>,
}
#[post("/form/reply", data = "<message>")]
pub async fn reply(
    _admin: Admin,
    message: Form<Strict<Reply>>,
    contest_client: &State<ContestClient>,
) -> Result<Redirect, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    let req = contest::AddMessageRequest {
        message: contest::Message {
            id: gen_uuid(),
            subject: message.subject.clone(),
            problem_id: message.problem_id,
            text: message.text.clone(),
            to: if Some(true) == message.broadcast {
                None
            } else {
                Some(message.user.clone())
            },
            from: None,
            sent_at: SystemTime::now().into(),
        },
    };
    match contest_client.add_message(tonic::Request::new(req)).await {
        Ok(_) => Ok(Redirect::to(uri!(templates::questions_template))),
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

use rocket::data::ToByteUnit;
pub async fn set_evaluation_file(
    problem_id: u64,
    file_language: String,
    file_content: Data<'_>,
    file_type: evaluation::evaluation_file::Type,
    evaluation_client: EvaluationClient,
) -> Result<Redirect, status::Custom<String>> {
    let stuff = evaluation::EvaluationFile {
        r#type: file_type as i32,
        source: protos::common::Source {
            lang: match file_language.as_str() {
                "Rust" => protos::common::ProgrammingLanguage::Rust as i32,
                "Cpp" => protos::common::ProgrammingLanguage::Cpp as i32,
                _ => protos::common::ProgrammingLanguage::None as i32,
            },
            code: file_content.open(512.mebibytes()).into_bytes().await.unwrap().value, // TODO: remove unwrap
        },
    };
    let req = evaluation::SetProblemEvaluationFileRequest {
        problem_id,
        command: Some(
            evaluation::set_problem_evaluation_file_request::Command::UpdateEvaluationFile(stuff), // TODO: unite update and add
        ),
    };
    match evaluation_client
        .set_problem_evaluation_file(tonic::Request::new(req))
        .await
    {
        Ok(_) => Ok(Redirect::to("/problem_files")),
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
        )),
    }
}

#[post("/form/set_checker/<problem_id>/<lang>", data = "<checker>")]
pub async fn set_checker(
    _admin: Admin,
    problem_id: u64,
    lang: String,
    checker: Data<'_>,
    evaluation_client: &State<EvaluationClient>,
) -> Result<Redirect, status::Custom<String>> {
    let evaluation_client = evaluation_client.inner().clone();
    set_evaluation_file(
        problem_id,
        lang,
        checker,
        evaluation::evaluation_file::Type::Checker,
        evaluation_client,
    )
    .await
}

#[post("/form/set_interactor/<problem_id>/<lang>", data = "<interactor>")]
pub async fn set_interactor(
    _admin: Admin,
    problem_id: u64,
    lang: String,
    interactor: Data<'_>,
    evaluation_client: &State<EvaluationClient>,
) -> Result<Redirect, status::Custom<String>> {
    let evaluation_client = evaluation_client.inner().clone();
    set_evaluation_file(
        problem_id,
        lang,
        interactor,
        evaluation::evaluation_file::Type::Interactor,
        evaluation_client,
    )
    .await
}
