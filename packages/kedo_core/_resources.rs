use crate::http::fetch::fetch::FetchClient;
use crate::streams::streams::{BoundedBufferChannelReader, InternalStreamResource};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

type ResourceId = u64;

pub struct StreamResourceTable {
    table: HashMap<ResourceId, Rc<RefCell<InternalStreamResource<Vec<u8>>>>>,
    next_id: ResourceId,
}

impl StreamResourceTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn add(&mut self, resource: InternalStreamResource<Vec<u8>>) -> ResourceId {
        let id = self.next_id;
        self.next_id += 1;
        self.table.insert(id, Rc::new(RefCell::new(resource)));
        id
    }

    pub fn get(
        &mut self,
        id: ResourceId,
    ) -> Option<Rc<RefCell<InternalStreamResource<Vec<u8>>>>> {
        self.table.get(&id).cloned()
    }

    pub fn remove(
        &mut self,
        id: ResourceId,
    ) -> Option<Rc<RefCell<InternalStreamResource<Vec<u8>>>>> {
        self.table.remove(&id)
    }

    pub fn reset(&mut self) {
        self.table.clear();
        self.next_id = 0;
    }
}

pub struct StreamResourceReaderTable {
    table: HashMap<ResourceId, Rc<RefCell<BoundedBufferChannelReader<Vec<u8>>>>>,
    next_id: ResourceId,
}

impl StreamResourceReaderTable {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn add(&mut self, reader: BoundedBufferChannelReader<Vec<u8>>) -> ResourceId {
        let id = self.next_id;
        self.next_id += 1;
        self.table.insert(id, Rc::new(RefCell::new(reader)));
        id
    }

    pub fn get(
        &mut self,
        id: ResourceId,
    ) -> Option<Rc<RefCell<BoundedBufferChannelReader<Vec<u8>>>>> {
        self.table.get(&id).cloned()
    }

    pub fn remove(
        &mut self,
        id: ResourceId,
    ) -> Option<Rc<RefCell<BoundedBufferChannelReader<Vec<u8>>>>> {
        self.table.remove(&id)
    }

    pub fn reset(&mut self) {
        self.table.clear();
        self.next_id = 0;
    }
}

pub struct ResourceTable {
    streams: StreamResourceTable, 
    readers: StreamResourceReaderTable,
    globals: GlobalResource,
}

impl ResourceTable {
    pub fn new() -> Self {
        Self {
            streams: StreamResourceTable::new(),
            readers: StreamResourceReaderTable::new(),
            globals: GlobalResource::new(),
        }
    }

    pub fn streams(&mut self) -> &mut StreamResourceTable {
        &mut self.streams
    }

    pub fn globals(&self) -> &GlobalResource {
        &self.globals
    }

    pub fn readers(&mut self) -> &mut StreamResourceReaderTable {
        &mut self.readers
    }
}
