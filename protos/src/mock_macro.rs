// overly complicated due to async-trait
macro_rules! rpc_mock_fn {
    ( $stname:ident, $fname:ident, $rname:ident, $in:ty, $out:ty ) => {
        fn $fname<'life0, 'async_trait>(
            &'life0 self,
            req: tonic::Request<$in>,
        ) -> core::pin::Pin<
            Box<
                (dyn core::future::Future<Output = Result<tonic::Response<$out>, tonic::Status>>
                     + Send
                     + 'async_trait),
            >,
        >
        where
            'life0: 'async_trait,
        {
            async fn f(
                _self: &$stname,
                req: tonic::Request<$in>,
            ) -> Result<tonic::Response<$out>, tonic::Status> {
                let res = match _self.$rname.clone() {
                    Ok(x) => Ok(tonic::Response::new(x)),
                    Err(x) => Err(tonic::Status::new(x.0, x.1)),
                };
                eprintln!(
                    "{}:\nreceived {:?}\nresponding{:?}",
                    std::any::type_name::<$stname>(),
                    req,
                    res
                );
                res
            }
            Box::pin(f(self, req))
        }
    };
}
macro_rules! rpc_mock_fn_streaming {
    ( $stname:ident, $fname:ident, $rname:ident, $in:ty, $out:ty) => {
        paste::paste!{
        fn $fname<'life0, 'async_trait>(
            &'life0 self,
            req: tonic::Request<$in>,
        ) -> core::pin::Pin<
            Box<
                (dyn core::future::Future<Output = Result<tonic::Response<Stream<$out>>,tonic::Status>>
                     + Send
                     + 'async_trait),
            >,
        >
        where
            'life0: 'async_trait,
        {
            async fn f(
                _self: &$stname,
                req: tonic::Request<$in>,
            ) -> Result<tonic::Response<Stream<$out>>,tonic::Status> {
                let res = _self.$rname.clone();
                eprintln!(
                    "{}:\nreceived {:?}\nresponding{:?}",
                    std::any::type_name::<$stname>(),
                    req,
                    res
                );
                match res {
                    Ok(x) => {
                        let stream = async_stream::try_stream! {
                            for m in x.iter() {
                                yield m.clone();
                            }
                        };
                        Ok(tonic::Response::new(Box::pin(stream) as Stream<$out>))
                    },
                    Err(x) => Err(tonic::Status::new(x.0, x.1))
                }
            }
            Box::pin(f(self, req))
        }
        }
    };
}


macro_rules! rpc_mock_setters {
    ( $fname:ident, $rname:ident, $in:ty, $out:ty ) => {
        paste::paste! {
            pub fn [<$fname _set>] (&mut self, val: $out) {
                self.$rname = Ok(val);
            }
            pub fn [<$fname _set_err>] (&mut self, val: tonic::Status) {
                self.$rname = Err((val.code(),val.message().to_string()));
            }
        }
    };
}

#[macro_export]
macro_rules! rpc_mock_server {
    ( $trait:ty; $stname:ident; $( ($fname:ident, $in:ty, $out:ty) ),*; $( ($sfname:ident, $sin:ty, $sout:ty) ),* ) => {
        paste::paste!{
            #[allow(dead_code)]
            type Stream<T> = core::pin::Pin<Box<dyn futures_core::Stream<Item = std::result::Result<T, tonic::Status>> + Send + Sync + 'static>>;
            #[derive(Debug, Clone)]
            pub struct $stname {
                $(
                    [<$fname _return>] : Result<$out,(tonic::Code,String)>,
                )*
                $(
                    [<$sfname _return>] : Result<Vec<$sout>,(tonic::Code,String)>,
                )*
            }
            impl $trait for $stname {
                $(
                    rpc_mock_fn!($stname, $fname, [<$fname _return>], $in, $out);
                )*
                $(
                    rpc_mock_fn_streaming!($stname, $sfname, [<$sfname _return>], $sin, $sout);
                    type [<$sfname Stream>] = Stream<$sout>;
                )*
            }
            impl $stname {
                $(
                    rpc_mock_setters!($fname,[<$fname _return>],$in,$out);
                )*
                $(
                    rpc_mock_setters!($sfname,[<$sfname _return>],$sin,Vec<$sout>);
                )*
            }
            impl Default for $stname {
                fn default() -> Self {
                    Self {
                        $(
                            [<$fname _return>] : Err((tonic::Code::Internal,String::from("the response for this mock method was not set"))),
                        )*
                        $(
                            [<$sfname _return>] : Err((tonic::Code::Internal,String::from("the response for this mock method was not set"))),
                        )*
                    }
                }
            }
        }
    }
}
