use hyper::Uri;


pub struct FetchRequest {
    pub uri: Uri,
    pub method: String,
    pub headers: Vec<(String, String)>,

    // TODO: body should be a Readable stream
    pub body: Option<String>,
}


