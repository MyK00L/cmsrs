#![feature(trait_alias)]

#[macro_use]
mod mock_macro;

pub mod common {
    tonic::include_proto!("common");
    impl From<std::time::Duration> for Duration {
        fn from(d: std::time::Duration) -> Self {
            Duration {
                secs: d.as_secs(),
                nanos: d.subsec_nanos(),
            }
        }
    }
    impl From<Duration> for std::time::Duration {
        fn from(d: Duration) -> std::time::Duration {
            std::time::Duration::new(d.secs, d.nanos)
        }
    }
    impl From<std::time::SystemTime> for Timestamp {
        fn from(t: std::time::SystemTime) -> Self {
            let d = t
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_else(|_| std::time::Duration::new(0, 0));
            Timestamp {
                secs: d.as_secs(),
                nanos: d.subsec_nanos(),
            }
        }
    }
    impl From<Timestamp> for std::time::SystemTime {
        fn from(t: Timestamp) -> std::time::SystemTime {
            let d = std::time::Duration::from(Duration {
                secs: t.secs,
                nanos: t.nanos,
            });
            std::time::SystemTime::UNIX_EPOCH + d
        }
    }
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
        (get_problem_statement,GetProblemRequest,GetProblemStatementResponse),
        (get_problem_info,GetProblemRequest,GetProblemInfoResponse),
        (get_announcement_list,GetAnnouncementListRequest,GetAnnouncementListResponse),
        (get_question_list,GetQuestionListRequest,GetQuestionListResponse),
        (set_user,SetUserRequest,SetUserResponse),
        (set_contest_metadata,SetContestMetadataRequest,SetContestMetadataResponse),
        (set_problem,SetProblemRequest,SetProblemResponse),
        (update_problem_info,UpdateProblemInfoRequest,SetProblemResponse),
        (update_problem_statement,UpdateProblemStatementRequest,SetProblemResponse),
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
        (get_problem,GetProblemRequest,GetProblemResponse),
        (get_contest,GetContestRequest,GetContestResponse),
        (set_contest,SetContestRequest,SetContestResponse),
        (get_testcase,GetTestcaseRequest,GetTestcaseResponse),
        (get_problem_testcases,GetProblemTestcasesRequest,GetProblemTestcasesResponse),
        (set_testcase,SetTestcaseRequest,SetTestcaseResponse),
        (get_problem_evaluation_file,GetProblemEvaluationFileRequest,GetProblemEvaluationFileResponse),
        (set_problem_evaluation_file,SetProblemEvaluationFileRequest,SetProblemEvaluationFileResponse)
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
