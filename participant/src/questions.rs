use protos::service::contest;

use rocket::http::Status;

use rocket::response::status;
use rocket::serde::Serialize;
use rocket::*;
use rocket_dyn_templates::Template;

use super::auth::User;
use super::clients::*;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MessageTemplate {
    is_question: bool,
    subject: String,
    problem: Option<u64>,
    text: String,
    timestamp: std::time::SystemTime,
}
impl MessageTemplate {
    fn from_announcement(m: contest::Message) -> Self {
        Self {
            is_question: false,
            subject: m.subject,
            problem: m.problem_id,
            text: m.text,
            timestamp: m.sent_at.into(),
        }
    }
    fn from_question(m: contest::Message) -> Self {
        Self {
            is_question: true,
            subject: m.subject,
            problem: m.problem_id,
            text: m.text,
            timestamp: m.sent_at.into(),
        }
    }
}
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct MessageListTemplate {
    messages: Vec<MessageTemplate>,
}

#[get("/questions")]
pub async fn questions(
    _user: User,
    contest_client: &State<ContestClient>,
) -> Result<Template, status::Custom<()>> {
    let mut cc0 = contest_client.inner().clone();
    let mut cc1 = contest_client.inner().clone();
    let (announcements, questions) = futures::join!(
        cc0.get_announcement_list(tonic::Request::new(
            contest::GetAnnouncementListRequest::default()
        )),
        cc1.get_question_list(tonic::Request::new(contest::GetQuestionListRequest {
            user_id: None, // Some(user.0) but rn this is a uint64 o.O
            ..Default::default()
        })),
    );
    drop(cc0);
    drop(cc1);
    let announcements = match announcements {
        Ok(response) => response.into_inner().announcements,
        Err(_) => {
            return Err(status::Custom(Status::InternalServerError, ()));
        }
    };
    let questions = match questions {
        Ok(response) => response.into_inner().questions,
        Err(_) => {
            return Err(status::Custom(Status::InternalServerError, ()));
        }
    };
    let mut messages: Vec<MessageTemplate> = questions
        .into_iter()
        .map(MessageTemplate::from_question)
        .chain(
            announcements
                .into_iter()
                .map(MessageTemplate::from_announcement),
        )
        .collect();
    messages.sort_by_key(|x| x.timestamp);
    messages.reverse();

    Ok(Template::render(
        "questions",
        MessageListTemplate { messages },
    ))
}
