use rocket::*;
use rocket_dyn_templates::Template;

#[cfg(feature = "mock")]
mod clients {
    use protos::service::contest;
    pub use protos::service::contest::contest_server::Contest;
    use protos::service::submission;
    use std::time::{Duration, SystemTime};
    // clients for testing
    pub type ContestClient = contest::MockContest;
    pub type SubmissionClient = submission::MockSubmission;
    use fake::{Fake, Faker};
    pub fn get_contest_client() -> ContestClient {
        let mut mock = contest::MockContest::default();

        let mut ql: contest::GetQuestionListResponse = Faker.fake();
        for q in ql.questions.iter_mut() {
            q.sent_at = SystemTime::now().into();
        }
        mock.get_question_list_set(ql);

        let mut al: contest::GetAnnouncementListResponse = Faker.fake();
        for a in al.announcements.iter_mut() {
            a.sent_at = SystemTime::now().into();
        }
        mock.get_announcement_list_set(al);

        mock.auth_user_set(contest::AuthUserResponse {
            response: Some(contest::auth_user_response::Response::Success(Faker.fake())),
        });

        mock.get_contest_metadata_set(contest::GetContestMetadataResponse {
            metadata: contest::ContestMetadata {
                name: String::from("contestname"),
                description: String::from("best contest ever"),
                start_time: Some((SystemTime::now()+Duration::from_secs(22)).into()),
                end_time: Some(
                    (SystemTime::now() + Duration::from_secs(3622)).into(),
                ),
            },
        });

        mock
    }
    pub fn get_submission_client() -> SubmissionClient {
        let mock = submission::MockSubmission::default();
        mock
    }
}

#[cfg(not(feature = "mock"))]
mod clients {
    use protos::service::contest;
    use protos::service::submission;
    // clients for production
    pub type ContestClient = contest::contest_client::ContestClient<tonic::transport::Channel>;
    pub type SubmissionClient =
        submission::submission_client::SubmissionClient<tonic::transport::Channel>;
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
mod common;
mod questions;

#[launch]
fn rocket() -> _ {
    let contest_client = clients::get_contest_client();
    let submission_client = clients::get_submission_client();
    rocket::build()
        .manage(contest_client)
        .manage(submission_client)
        .mount(
            "/",
            routes![
                auth::root,
                auth::root_logged,
                auth::not_logged_redirect,
                auth::login,
                auth::logout,
                questions::questions,
                questions::post_question,
            ],
        )
        .attach(Template::fairing())
}
