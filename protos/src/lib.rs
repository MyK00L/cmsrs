#![feature(trait_alias)]

/*
 *rpc_mock_server!(super::service::test::test_server::Test; MockTest; (log_string, super::service::test::LogRequest, super::service::test::LogResponse),  (test_string, super::service::test::StringRequest, super::service::test::StringResponse)  );
 * */

#[macro_use]
mod mock_macro;

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
    use std::error::Error;
    use std::fmt::Debug;
    use tonic::body::BoxBody;
    use tonic::client::GrpcService;
    use tonic::codegen::Body;
    use tonic::transport::Channel;

    pub trait ChannelTrait = GrpcService<BoxBody> + 'static + Sync + Send + Debug + Clone
    where
        <Self as GrpcService<BoxBody>>::ResponseBody: Send + Sync + 'static,
        <<Self as GrpcService<BoxBody>>::ResponseBody as Body>::Error:
            Into<Box<(dyn Error + Send + Sync + 'static)>> + Send,
        <Self as GrpcService<BoxBody>>::Future: Send;

    pub enum Service {
        CONTEST,
        DISPATCHER,
        EVALUATION,
        SUBMISSION,
        WORKER,
        TEST,
    }

    pub fn get_new_channel(s: Service) -> Channel {
        Channel::from_static(get_remote_address(s))
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
