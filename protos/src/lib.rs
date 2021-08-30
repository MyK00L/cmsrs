#![feature(trait_alias)]

pub use prost_types;

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
pub mod worker {
    tonic::include_proto!("worker");
}
pub mod service {
    pub mod contest {
        tonic::include_proto!("service.contest");
        rpc_mock_server!(contest_server::Contest; MockContest;
        (auth_user,AuthUserRequest,AuthUserResponse),
        (get_contest_metadata,GetContestMetadataRequest,GetContestMetadataResponse),
        (get_problem,GetProblemRequest,GetProblemResponse),
        (get_announcement_list,GetAnnouncementListRequest,GetAnnouncementListResponse),
        (get_question_list,GetQuestionListRequest,GetQuestionListResponse),
        (set_user,SetUserRequest,SetUserResponse),
        (set_contest_metadata,SetContestMetadataRequest,SetContestMetadataResponse),
        (set_problem,SetProblemRequest,SetProblemResponse),
        (add_message,AddMessageRequest,AddMessageResponse)
        );
    }
    pub mod dispatcher {
        tonic::include_proto!("service.dispatcher");
        rpc_mock_server!(dispatcher_server::Dispatcher; MockDispatcher;
        (evaluate_submission,EvaluateSubmissionRequest,EvaluateSubmissionResponse)
        );
    }
    pub mod evaluation {
        tonic::include_proto!("service.evaluation");
        rpc_mock_server!(evaluation_server::Evaluation; MockEvaluation;
        (get_user_scoring,GetUserScoringRequest,GetUserScoringResponse),
        (set_user_scoring,SetUserScoringRequest,SetUserScoringResponse),
        (get_problem,GetProblemRequest,GetProblemResponse),
        (set_problem,SetProblemRequest,SetProblemResponse),
        (get_testcase,GetTestcaseRequest,GetTestcaseResponse),
        (set_testcase,SetTestcaseRequest,SetTestcaseResponse),
        (get_evaluation_file,GetEvaluationFileRequest,GetEvaluationFileResponse),
        (set_evaluation_file,SetEvaluationFileRequest,SetEvaluationFileResponse)
        );
    }
    pub mod submission {
        tonic::include_proto!("service.submission");
        rpc_mock_server!(submission_server::Submission; MockSubmission;
        (evaluate_submission,EvaluateSubmissionRequest,EvaluateSubmissionResponse),
        (get_submission_list,GetSubmissionListRequest,GetSubmissionListResponse),
        (get_submission_details,GetSubmissionDetailsRequest,GetSubmissionDetailsResponse)
        );
    }
    pub mod worker {
        tonic::include_proto!("service.worker");
        rpc_mock_server!(worker_server::Worker; MockWorker;
        (evaluate_submission,EvaluateSubmissionRequest,EvaluateSubmissionResponse),
        (update_testcase,UpdateTestcaseRequest,UpdateTestcaseResponse),
        (update_source,UpdateSourceRequest,UpdateSourceResponse)
        );
    }
    pub mod test {
        tonic::include_proto!("service.test");
        rpc_mock_server!(test_server::Test; MockTest;
        (test_string,TestStringRequest,TestStringResponse),
        (log_string,LogStringRequest,LogStringResponse));
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
