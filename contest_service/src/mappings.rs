use std::convert::TryFrom;

use super::utils;
use mongodb::bson::Document;
use protos::service::contest::GetContestMetadataResponse;
use tonic::Response;

#[derive(Debug)]
pub enum MappingError {
    MissingField(&'static str),
}

pub mod contest {
    use super::*;
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

    impl TryFrom<protos::service::contest::SetContestMetadataRequest> for ContestMetadata {
        fn try_from(
            pb_meta: protos::service::contest::SetContestMetadataRequest,
        ) -> Result<Self, Self::Error> {
            let metadata = pb_meta
                .metadata
                .ok_or(MappingError::MissingField("metadata"))?;
            Ok(Self {
                name: metadata.name,
                description: metadata.description,
                start_time: metadata.start_time.map(utils::prost_ts_to_systime),
                end_time: metadata.end_time.map(utils::prost_ts_to_systime),
            })
        }

        type Error = MappingError;
    }

    impl From<ContestMetadata> for Document {
        fn from(m: ContestMetadata) -> Self {
            let mut result = Document::new();
            result.insert("name", m.name);
            result.insert("description", m.description);
            result.insert("startTime", m.start_time.map(utils::systime_to_timestamp));
            result.insert("endTime", m.end_time.map(utils::systime_to_timestamp));
            result
        }
    }
}
