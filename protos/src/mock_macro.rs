

// overly complicated due to async-trait
#[macro_export]
macro_rules! rpc_mock_fn {
    ( $stname:ident, $fname:ident, $rname:ident, $in:ty, $out:ty ) => {
        fn $fname<'life0, 'async_trait>(&'life0 self, req: tonic::Request<$in>) -> core::pin::Pin<Box<(dyn core::future::Future<Output = Result<tonic::Response<$out>, tonic::Status>> + Send + 'async_trait)>>
        where
        'life0: 'async_trait
        {
            async fn f(_self: &$stname, _req: tonic::Request<$in>) -> Result<tonic::Response<$out>,tonic::Status> {
                Ok(tonic::Response::new(_self.$rname.clone()))
            }
            Box::pin(f(self,req))
        }
    };
}

#[macro_export]
macro_rules! rpc_mock_set_fn {
    ( $fname:ident, $rname:ident, $out:ty) => {
        pub fn $fname(&mut self, val: $out) {
            self.$rname = val;
        }
    };
}

#[macro_export]
macro_rules! rpc_mock_server {
    ( $trait:ty; $stname:ident; $( ($fname:ident, $in:ty, $out:ty) ),* ) => {
        paste::paste!{
            pub struct $stname {
                $(
                    [<$fname _return>] : $out,
                )*
            }
            impl $trait for $stname {
                $(
                    rpc_mock_fn!($stname, $fname, [<$fname _return>], $in, $out);
                )*
            }
            impl $stname {
                $(
                    rpc_mock_set_fn!([<$fname _set>], [<$fname _return>], $out);
                )*
            }
        }
    }
}
