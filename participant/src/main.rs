use rocket::*;
use rocket_dyn_templates::Template;

#[cfg(feature = "mock")]
mod clients {
    use protos::service::contest;
    pub use protos::service::contest::contest_server::Contest;
    use protos::service::submission;
    pub use protos::service::submission::submission_server::Submission;
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

        let secs_until_start = 10;
        mock.get_contest_metadata_set(contest::GetContestMetadataResponse {
            metadata: contest::ContestMetadata {
                name: String::from("contestname"),
                description: String::from("best contest ever"),
                start_time: Some(
                    (SystemTime::now() + Duration::from_secs(secs_until_start)).into(),
                ),
                end_time: Some(
                    (SystemTime::now() + Duration::from_secs(3600 + secs_until_start)).into(),
                ),
            },
            problems: Faker.fake(),
        });

        mock
    }
    pub fn get_submission_client() -> SubmissionClient {
        let mut mock = submission::MockSubmission::default();
        mock.get_submission_list_set(Faker.fake());
        mock.get_submission_details_set(submission::GetSubmissionDetailsResponse {
            sub: protos::evaluation::Submission {
                user: String::from("hi"),
                problem_id: 2,
                source: protos::common::Source {
                    code: "#define OII\nint main(){\n\treturn 0;\n}\n/*\n<b>a</b>\n*/\n"
                        .as_bytes()
                        .to_vec(),
                    lang: protos::common::ProgrammingLanguage::Cpp as i32,
                },
            },
            state: protos::service::submission::SubmissionState::Evaluated as i32,
            res: Some(protos::evaluation::EvaluationResult {
                compilation_result: protos::evaluation::CompilationResult {
                    outcome: protos::evaluation::compilation_result::Outcome::Success as i32,
                    ..Default::default()
                },
                subtask_results: vec![
                    protos::evaluation::SubtaskResult {
                        testcase_results: vec![
                            protos::evaluation::TestcaseResult {
                                outcome: protos::evaluation::testcase_result::Outcome::Ok as i32,
                                score: Faker.fake(),
                                ..Default::default()
                            };
                            9
                        ],
                        score: Faker.fake(),
                        ..Default::default()
                    };
                    5
                ],
                score: Faker.fake(),
            }),
        });
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
mod problems;
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
                auth::statics,
                auth::not_logged_redirect,
                auth::login,
                auth::logout,
                questions::questions,
                questions::post_question,
                problems::problems,
                problems::submit,
                problems::submission_details_template,
            ],
        )
        .attach(Template::fairing())
}
