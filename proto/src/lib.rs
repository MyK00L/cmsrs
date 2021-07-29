use tonic::{transport::Server, Request, Response, Status};

pub mod common {
    tonic::include_proto!("common");
}
pub mod contest_service {
    tonic::include_proto!("contest_service");
}
pub mod dispatcher_service {
    tonic::include_proto!("dispatcher_service");
}
pub mod evaluation_files_service {
    tonic::include_proto!("evaluation_files_service");
}
pub mod evaluation_info {
    tonic::include_proto!("evaluation_info");
}
pub mod submission {
    tonic::include_proto!("submission");
}
pub mod submission_service {
    tonic::include_proto!("submission_service");
}
pub mod user_info {
    tonic::include_proto!("user_info");
}
pub mod worker_service {
    tonic::include_proto!("worker_service");
}

#[cfg(test)]
mod tests {
	#[test]
	fn it_works() {
		assert_eq!(2 + 2, 4);
	}
}
