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
        Redirect::to("/home")
    } else {
        Redirect::to(uri!(root))
    }
}

#[derive(FromForm)]
pub struct UpdateContest {
    name: String,
    description: String,
    start_time: String,
    end_time: String,
}
#[post("/form/update_contest", data = "<contest>")]
pub async fn update_contest(
    _admin: Admin,
    contest: Form<Strict<UpdateContest>>,
    contest_client: &State<ContestClient>,
) -> Result<Redirect, status::Custom<String>> {
    let contest_client = contest_client.inner().clone();
    let req = contest::SetContestMetadataRequest {
        metadata: contest::ContestMetadata {
            name: contest.name.clone(),
            description: contest.description.clone(),
            start_time: chrono::prelude::Utc
                .datetime_from_str(&contest.start_time, "%FT%T")
                .ok()
                .map(|t| SystemTime::from(t).into()),
            end_time: chrono::prelude::Utc
                .datetime_from_str(&contest.end_time, "%FT%T")
                .ok()
                .map(|t| SystemTime::from(t).into()),
        },
    };
    match contest_client
        .set_contest_metadata(tonic::Request::new(req))
        .await
    {
        Ok(_) => Ok(Redirect::to("/contest")),
        Err(err) => Err(status::Custom(
            Status::InternalServerError,
            format!("Error in rpc request:\n{:?}", err),
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
