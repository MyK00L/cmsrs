#![feature(trait_alias)]

pub mod common {
    tonic::include_proto!("common");
}
pub mod evaluation {
    tonic::include_proto!("evaluation");
}
pub mod scoring {
    tonic::include_proto!("scoring");
}
pub mod user {
    tonic::include_proto!("user");
}
pub mod worker {
    tonic::include_proto!("worker");
}
pub mod service {
    pub mod contest {
        tonic::include_proto!("service.contest");
    }
    pub mod dispatcher {
        tonic::include_proto!("service.dispatcher");
    }
    pub mod evaluation_files {
        tonic::include_proto!("service.evaluation_files");
    }
    pub mod submission {
        tonic::include_proto!("service.submission");
    }
    pub mod worker {
        tonic::include_proto!("service.worker");
    }
    pub mod test {
        tonic::include_proto!("service.test");
    }
}

pub trait ChannelTrait = tonic::client::GrpcService<tonic::body::BoxBody>+Sync where
<Self as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody: Send + Sync + 'static,
    <<Self as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody as tonic::codegen::Body>::Error:
        Into<Box<(dyn std::error::Error + Send + Sync + 'static)>> + Send;
