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

pub mod utils {
    pub trait ChannelTrait = tonic::client::GrpcService<tonic::body::BoxBody>+'static+Sync+Send+std::fmt::Debug+Clone where
<Self as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody: Send + Sync + 'static,
    <<Self as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody as tonic::codegen::Body>::Error:
        Into<Box<(dyn std::error::Error + Send + Sync + 'static)>> + Send,
<Self as tonic::client::GrpcService<tonic::body::BoxBody>>::Future: Send;

    pub enum Service {
        CONTEST,
        DISPATCHER,
        EVALUATION,
        SUBMISSION,
        WORKER,
        TEST,
    }

    use super::service::*;
    #[derive(Debug, Clone)]
    pub struct ClientManager<T: ChannelTrait> {
        pub contest_client: contest::contest_client::ContestClient<T>,
        pub dispatcher_client: dispatcher::dispatcher_client::DispatcherClient<T>,
        pub evaluation_client: evaluation_files::evaluation_client::EvaluationClient<T>,
        pub submission_client: submission::submission_client::SubmissionClient<T>,
        pub worker_client: worker::worker_client::WorkerClient<T>,
        pub test_client: test::test_client::TestClient<T>,
    }
    impl Default for ClientManager<tonic::transport::Channel> {
        fn default() -> Self {
            ClientManager {
                contest_client: contest::contest_client::ContestClient::new(get_new_channel(
                    Service::CONTEST,
                )),
                dispatcher_client: dispatcher::dispatcher_client::DispatcherClient::new(
                    get_new_channel(Service::DISPATCHER),
                ),
                evaluation_client: evaluation_files::evaluation_client::EvaluationClient::new(
                    get_new_channel(Service::EVALUATION),
                ),
                submission_client: submission::submission_client::SubmissionClient::new(
                    get_new_channel(Service::SUBMISSION),
                ),
                worker_client: worker::worker_client::WorkerClient::new(get_new_channel(
                    Service::WORKER,
                )),
                test_client: test::test_client::TestClient::new(get_new_channel(Service::TEST)),
            }
        }
    }

    pub fn get_new_channel(s: Service) -> tonic::transport::Channel {
        tonic::transport::Channel::from_static(get_remote_address(s))
            .connect_lazy()
            .unwrap()
    }

    #[cfg(not(feature = "loopback"))]
    pub fn get_local_address(_s: Service) -> &'static str {
        "0.0.0.0:50051"
    }
    #[cfg(not(feature = "loopback"))]
    pub fn get_remote_address(s: Service) -> &'static str {
        match s {
            Service::TEST => "http://rpc-server:50051",
            _ => "a",
        }
    }
    #[cfg(feature = "loopback")]
    pub fn get_local_address(_s: Service) -> &'static str {
        "127.0.0.1:50051"
    }
    #[cfg(feature = "loopback")]
    pub fn get_remote_address(_s: Service) -> &'static str {
        "http://127.0.0.1:50051"
    }
}
