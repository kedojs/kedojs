use crate::http::fetch::FetchClient;

pub struct GlobalResource {
    fetch_client: FetchClient,
}

impl GlobalResource {
    pub fn new() -> Self {
        Self {
            fetch_client: FetchClient::new(),
        }
    }

    pub fn fetch_client(&self) -> &FetchClient {
        &self.fetch_client
    }
}
