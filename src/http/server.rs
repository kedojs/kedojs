// use boa_engine::JsObject;
// use hyper::{service::Service, Request, Response};

#[derive(Debug, Clone)]
struct KedoService {
  // callback: JsObject,
}

// impl Service<Request<IncomingBody>> for KedoService {
//     type Response = Response<Full<Bytes>>;
//     type Error = hyper::Error;
//     type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

//     fn call(&self, req: Request<IncomingBody>) -> Self::Future {
//         fn mk_response(s: String) -> Result<Response<Full<Bytes>>, hyper::Error> {
//             Ok(Response::builder().body(Full::new(Bytes::from(s))).unwrap())
//         }

//         if req.uri().path() != "/favicon.ico" {
//             *self.counter.lock().expect("lock poisoned") += 1;
//         }

//         let res = match req.uri().path() {
//             "/" => mk_response(format!("home! counter = {:?}", self.counter)),
//             "/posts" => mk_response(format!("posts, of course! counter = {:?}", self.counter)),
//             "/authors" => mk_response(format!(
//                 "authors extraordinare! counter = {:?}",
//                 self.counter
//             )),
//             // Return the 404 Not Found for other routes, and don't increment counter.
//             _ => return Box::pin(async { mk_response("oh no! not found".into()) }),
//         };

//         Box::pin(async { res })
//     }
// }
