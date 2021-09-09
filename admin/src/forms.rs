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

#[derive(FromForm, Debug)]
pub struct UserScoringMethod {
    aggregation: String,
    score_weight: f64,
    wrong_submission_count_weight: f64,
    time_secs_weight: f64,
}

#[post("/form/update_contest", data = "<contest>")]
pub async fn update_contest(
    _admin: Admin,
    contest: Form<Strict<templates::ContestTemplate>>,
    _contest_client: &State<ContestClient>,
    _evaluation_client: &State<EvaluationClient>,
) -> Result<Redirect, status::Custom<String>> {
    eprintln!("{:?}", contest);
    Ok(Redirect::to("/contest"))
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
