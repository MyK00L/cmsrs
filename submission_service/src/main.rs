use std::time::SystemTime;

use futures::stream::StreamExt;

use convert::mongo::*;

use mongodb::{Client, bson::{Document, doc, spec::ElementType}, options::{ClientOptions, FindOptions}};
use protos::common::*;
use protos::service::submission::*;
use protos::service::submission::submission_server::*;
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};

#[cfg(test)]
mod tests;

// TODO: remove credentials to connect to db.
const CONNECTION_STRING: &str = "mongodb://root:example@submission_service_db:27017/";

fn internal_error<T>(e: T) -> Status
where
    T: std::fmt::Debug,
{
    Status::internal(format!("{:?}", e))
}

#[derive(Debug)]
pub struct SubmissionService {
    db_client: Client,
}

impl SubmissionService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            db_client: Client::with_options(ClientOptions::parse(CONNECTION_STRING).await?)?,
        })
    }

    fn get_collection(&self) -> mongodb::Collection<Document> {
        self.db_client
            .database("submissionsdb")
            .collection::<Document>("submissions")
    }
}

// TODO check this
fn get_item_from_doc(doc: Document) -> get_submission_list_response::Item {
    get_submission_list_response::Item{
        submission_id: Some(protos::common::Id{
            rand_id: doc.get_i64("_id").unwrap() as u64
        }),
        author_username: doc.get_str("authorUsername").unwrap().to_string(),
        problem_id: doc.get_i32("problemId").unwrap() as u32,
        timestamp: Some(
            timestamp_to_systime(doc.get_timestamp("created").unwrap()).into()
        ),
        overall_score: {
            let opt_score = doc.get("overallScore");
            if let Some(score) = opt_score {
                if score.element_type() == ElementType::Double {
                    Some(get_submission_list_response::item::OverallScore::OverallScoreDouble(score.as_f64().unwrap()))
                } else {
                    Some(get_submission_list_response::item::OverallScore::OverallScoreBool(score.as_bool().unwrap()))
                }
            } else {
                None
            }
        }
    }
}

#[tonic::async_trait]
impl Submission for SubmissionService {
    async fn evaluate_submission(
        &self, 
        request: Request<EvaluateSubmissionRequest>
    ) -> Result<Response<EvaluateSubmissionResponse>, Status> { 
        let evaluate_submission_request = request.into_inner();
        let _submission = evaluate_submission_request.sub.clone();
        // 1) write into dabatase with Pending state
	    // 2) redirect request to the dispatcher and await response
	    // 3) write values returned by the dispatcher into database
	    //    changing the state to Evaluated
        todo!() 
    }

    /*
    get_submission_list --> based on the author, we keep an index on authorUsername
    */
    async fn get_submission_list(
        &self,
        request: Request<GetSubmissionListRequest>
    ) -> Result<Response<GetSubmissionListResponse>, Status> { 
        let list_request = request.into_inner();
        let opt_limit = list_request.limit.clone();
        let opt_author_username = list_request.author_username.clone();
        let opt_problem_id = list_request.problem_id.clone();

        let doc_filter;
        if opt_author_username.is_some() && opt_problem_id.is_some() {
            doc_filter = doc! {
                "authorUsername": opt_author_username.unwrap(),
                "problemId": opt_problem_id.unwrap(),
            };
        } else if let Some(author_username) =  opt_author_username {
            doc_filter = doc! {
                "authorUsername": author_username,
            };
        } else if opt_problem_id.is_some() {
            doc_filter = doc! {
                "problemId": opt_problem_id.unwrap(),
            };
        } else {
            doc_filter = doc! {};
        }

        let submissions = self
            .get_collection()
            .find(
                doc_filter,
                FindOptions::builder().limit(
                    if let Some(limit) = opt_limit {
                        Some(limit as i64)
                    } else {
                        None
                    }).build()
            )
            .await
            .map_err(internal_error)?
            .filter(|opt_submission| futures::future::ready(opt_submission.is_ok()))
            .map(|some_submission| { 
                match some_submission {
                    Ok(submission) => Some(get_item_from_doc(submission)),
                    Err(_) => None
                }
            })
            .filter(|opt_item| futures::future::ready(opt_item.is_some()))
            .map(|some_item| some_item.unwrap())
            .collect::<Vec<_>>()
            .await;

        Ok(Response::new(
            GetSubmissionListResponse {
                list: submissions
            }
        ))
    }

    async fn get_submission_details(
        &self,
        _request: Request<GetSubmissionDetailsRequest>
    ) -> Result<Response<GetSubmissionDetailsResponse>, Status> { 
        todo!() 
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_local_address(Service::CONTEST).parse()?;
    let submission_service = SubmissionService::new().await?;

    println!("Starting submission server");
    Server::builder()
        .add_service(SubmissionServer::new(submission_service))
        .serve(addr)
        .await?;
    Ok(())
}