use rocket::*;
use rocket_dyn_templates::Template;

#[cfg(test)]
mod clients {
    // clients for testing
    pub type ContestClient = contest::MockContest;
    pub type SubmissionClient = submission::MockSubmission;
    pub fn get_contest_client() -> ContestClient {
        ContestClient::default()
    }
    pub fn get_submission_client() -> SubmissionClient {
        SubmissionClient::default()
    }
}

#[cfg(not(test))]
mod clients {
    // clients for production
    pub type ContestClient =
        protos::service::contest::contest_client::ContestClient<tonic::transport::Channel>;
    pub type SubmissionClient =
        protos::service::submission::submission_client::SubmissionClient<tonic::transport::Channel>;
    pub fn get_contest_client() -> ContestClient {
        ContestClient::new(protos::utils::get_new_channel(
            protos::utils::Service::CONTEST,
        ))
    }
    pub fn get_submission_client() -> SubmissionClient {
        SubmissionClient::new(protos::utils::get_new_channel(
            protos::utils::Service::SUBMISSION,
        ))
    }
}

mod auth;

#[launch]
fn rocket() -> _ {
    let contest_client = clients::get_contest_client();
    let submission_client = clients::get_submission_client();
    rocket::build()
        .manage(contest_client)
        .manage(submission_client)
        .mount(
            "/",
            routes![auth::root, auth::root_logged, auth::login, auth::logout],
        )
        .attach(Template::fairing())
}
