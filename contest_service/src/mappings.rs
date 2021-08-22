use super::utils;
use mongodb::bson::Document;
use protos::service::contest::GetContestMetadataResponse;
use tonic::Response;

pub struct ContestMetadata {
    name: String,
    description: String,
    start_time: Option<std::time::SystemTime>,
    end_time: Option<std::time::SystemTime>,
}
impl From<Document> for ContestMetadata {
    fn from(value: Document) -> Self {
        Self {
            name: value.get("name").unwrap().to_string(),
            description: value.get("description").unwrap().to_string(),
            start_time: value
                .get("startTime")
                .map(|x| x.as_timestamp().map(utils::timestamp_to_systime).unwrap()),
            end_time: value
                .get("endTime")
                .map(|x| x.as_timestamp().map(utils::timestamp_to_systime).unwrap()),
        }
    }
}
impl From<ContestMetadata> for Response<GetContestMetadataResponse> {
    fn from(md: ContestMetadata) -> Self {
        Response::new(GetContestMetadataResponse {
            metadata: Some(protos::service::contest::ContestMetadata {
                name: md.name,
                description: md.description,
                start_time: md.start_time.map(utils::systime_to_prost_ts),
                end_time: md.end_time.map(utils::systime_to_prost_ts),
            }),
        })
    }
}
