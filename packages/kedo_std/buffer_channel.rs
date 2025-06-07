use futures::Stream;
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StreamError {
    Closed,
    ChannelFull,
    SendError(String),
    ReceiverTaken,
    Empty,
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamError::Closed => write!(f, "Stream is closed"),
            StreamError::ChannelFull => write!(f, "Channel is full"),
            StreamError::ReceiverTaken => write!(f, "Receiver is taken"),
            StreamError::SendError(s) => write!(f, "Send error: {}", s),
            StreamError::Empty => write!(f, "Stream is empty"),
        }
    }
}

impl Into<f64> for StreamError {
    fn into(self) -> f64 {
        match self {
            StreamError::Closed => -1.0,
            StreamError::ChannelFull => -2.0,
            StreamError::ReceiverTaken => -3.0,
            StreamError::SendError(_) => -4.0,
            StreamError::Empty => -5.0,
        }
    }
}

/// |--------------------------------------------------------------------------|
/// |                           StreamCompletion                               |
/// |--------------------------------------------------------------------------|
/// A wrapper struct for managing stream completion state.
///
/// `StreamCompletion` provides a reference-counted wrapper around the inner
/// completion state, allowing multiple owners to track and modify the completion
/// status of a stream.
///
/// # Examples
///
/// ```
/// use kedo_std::StreamCompletion;
///
/// let completion = StreamCompletion::new();
/// ```
#[derive(Debug, Clone, Default)]
pub struct StreamCompletion {
    inner: std::rc::Rc<RefCell<StreamCompletionInner>>,
}

#[derive(Debug, Default)]
pub struct StreamCompletionInner {
    closed: bool,
    waker: Option<std::task::Waker>,
}

impl StreamCompletion {
    pub fn close(&mut self) {
        let mut mut_ref = self.inner.borrow_mut();
        mut_ref.closed = true;
        if let Some(waker) = mut_ref.waker.take() {
            waker.wake();
        }
    }

    pub fn is_closed(&self) -> bool {
        self.inner.borrow_mut().closed
    }

    pub fn set_waker(&mut self, waker: std::task::Waker) {
        self.inner.borrow_mut().waker = Some(waker);
    }
}

impl Future for StreamCompletion {
    type Output = Result<(), StreamError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_closed() {
            Poll::Ready(Ok(()))
        } else {
            let self_mut = unsafe { self.get_unchecked_mut() };
            self_mut.set_waker(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub trait BufferChannel<R, W> {
    fn acquire_reader(&mut self) -> Option<R>;
    fn acquire_writer(&mut self) -> Option<W>;
    fn writer(&self) -> Option<&W>;
    fn close(&mut self);
    fn completion(&self) -> StreamCompletion;
}

pub trait BufferChannelReader<T> {
    fn try_read(&mut self) -> Result<T, StreamError>;
    fn read(&mut self) -> impl Future<Output = Result<T, StreamError>>;
}

pub trait BufferChannelWriter<T> {
    fn try_write(&self, item: T) -> Result<(), StreamError>;
    fn write(&self, item: T) -> impl Future<Output = Result<(), StreamError>>;
}

/// |--------------------------------------------------------------------------|
/// |                       BoundedBufferChannel                               |
/// |--------------------------------------------------------------------------|
/// A bounded buffer channel implementation that wraps Tokio's MPSC channel.
///
/// This struct provides a message-passing channel with a fixed buffer size,
/// allowing communication between different parts of async code.
///
/// # Type Parameters
///
/// * `T` - The type of values being sent through the channel
///
/// # Fields
///
/// * `sender` - Optional sender half of the channel
/// * `receiver` - Optional receiver half of the channel
/// * `completion` - Tracks the completion state of the stream
///
/// # Examples
///
/// ```
/// use kedo_std::BoundedBufferChannel;
///
/// let channel: BoundedBufferChannel<i32> = BoundedBufferChannel::new(100);
/// ```
/// TODO: use buffer size to implement backpressure
pub struct BoundedBufferChannel<T> {
    sender: Option<BoundedBufferChannelWriter<T>>,
    receiver: Option<BoundedBufferChannelReader<T>>,
    completion: StreamCompletion,
}

impl<T> Drop for BoundedBufferChannel<T> {
    fn drop(&mut self) {
        self.close();
    }
}

impl<T> BoundedBufferChannel<T> {
    /// Creates a new bounded buffer channel with the specified capacity.
    ///
    /// A bounded buffer channel is a channel that can hold a limited number of items
    /// before blocking on writes. This implementation uses tokio's mpsc channel internally.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The maximum number of items that can be stored in the channel buffer
    ///
    /// # Returns
    ///
    /// Returns a new `BoundedBufferChannel<T>` instance with the specified capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// let channel = BoundedBufferChannel::<i32>::new(10); // Creates a channel that can hold up to 10 items
    /// ```
    pub fn new(capacity: usize) -> Self {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity);

        Self {
            sender: Some(BoundedBufferChannelWriter { sender }),
            receiver: Some(BoundedBufferChannelReader { receiver }),
            completion: StreamCompletion::default(),
        }
    }
}

impl<T> BufferChannel<BoundedBufferChannelReader<T>, BoundedBufferChannelWriter<T>>
    for BoundedBufferChannel<T>
{
    fn acquire_reader(&mut self) -> Option<BoundedBufferChannelReader<T>> {
        self.receiver.take()
    }

    fn acquire_writer(&mut self) -> Option<BoundedBufferChannelWriter<T>> {
        // lets make a copy of the sender
        let sender = self.sender.as_ref()?;
        let sender = sender.sender.clone();
        Some(BoundedBufferChannelWriter { sender })
    }

    fn writer(&self) -> Option<&BoundedBufferChannelWriter<T>> {
        self.sender.as_ref()
    }

    fn close(&mut self) {
        let _ = self.sender.take();
        self.completion.close();
    }

    fn completion(&self) -> StreamCompletion {
        return self.completion.clone();
    }
}

/// A writer handle for the bounded buffer channel.
///
/// This struct provides a way to write values to the channel from multiple locations
/// while maintaining the channel's bounded characteristics.
///
/// # Type Parameters
///
/// * `T` - The type of values being sent through the channel
pub struct BoundedBufferChannelWriter<T> {
    sender: tokio::sync::mpsc::Sender<T>,
}

impl<T> BufferChannelWriter<T> for BoundedBufferChannelWriter<T> {
    /// Asynchronously writes an item to the channel.
    ///
    /// This method will block if the channel is at capacity until space becomes available
    /// or the channel is closed.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to send through the channel
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the item was successfully sent, or `StreamError::Closed`
    /// if the channel is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// let channel = BoundedBufferChannel::<i32>::new(1);
    /// let writer = channel.writer().unwrap();
    /// if let Err(e) = writer.write(42).await {
    ///     println!("Failed to write: {}", e);
    /// }
    /// ```
    async fn write(&self, item: T) -> Result<(), StreamError> {
        self.sender
            .send(item)
            .await
            .map_err(|_| StreamError::Closed)
    }

    fn try_write(&self, item: T) -> Result<(), StreamError> {
        self.sender.try_send(item).map_err(|e| match e {
            tokio::sync::mpsc::error::TrySendError::Full(_) => StreamError::ChannelFull,
            tokio::sync::mpsc::error::TrySendError::Closed(_) => StreamError::Closed,
        })
    }
}

/// A reader handle for the bounded buffer channel.
///
/// This struct provides methods to read values from the channel and can be used
/// as a Stream implementation for working with async iterators.
///
/// # Type Parameters
///
/// * `T` - The type of values being received from the channel
#[derive(Debug)]
pub struct BoundedBufferChannelReader<T> {
    receiver: tokio::sync::mpsc::Receiver<T>,
}

impl<T> BufferChannelReader<T> for BoundedBufferChannelReader<T> {
    /// Attempts to read an item from the channel without blocking.
    ///
    /// This method will immediately return an error if the channel is empty
    /// or closed, rather than waiting for a value.
    ///
    /// # Returns
    ///
    /// Returns `Ok(T)` with the received item if successful, or a `StreamError` if:
    /// - The channel is empty (`StreamError::Empty`)
    /// - The channel is closed (`StreamError::Closed`)
    ///
    /// # Examples
    ///
    /// ```
    /// let mut channel = BoundedBufferChannel::<i32>::new(1);
    /// let mut reader = channel.aquire_reader().unwrap();
    /// match reader.try_read() {
    ///     Ok(value) => println!("Read value: {}", value),
    ///     Err(e) => println!("Error reading: {}", e),
    /// }
    /// ```
    fn try_read(&mut self) -> Result<T, StreamError> {
        self.receiver.try_recv().map_err(|e| match e {
            tokio::sync::mpsc::error::TryRecvError::Empty => StreamError::Empty,
            tokio::sync::mpsc::error::TryRecvError::Disconnected => StreamError::Closed,
        })
    }

    /// Asynchronously reads an item from the channel.
    ///
    /// This method will wait for an item to become available if the channel
    /// is empty. If the channel is closed and empty, it returns an error.
    ///
    /// # Returns
    ///
    /// Returns `Ok(T)` with the received item if successful, or `StreamError::Closed`
    /// if the channel is closed and no more items are available.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut channel = BoundedBufferChannel::<i32>::new(1);
    /// let mut reader = channel.aquire_reader().unwrap();
    /// match reader.read().await {
    ///     Ok(value) => println!("Read value: {}", value),
    ///     Err(e) => println!("Error reading: {}", e),
    /// }
    /// ```
    async fn read(&mut self) -> Result<T, StreamError> {
        tokio::select! {
            biased;
            msg = self.receiver.recv() => msg.ok_or(StreamError::Closed),
        }
    }
}

impl<T> Stream for BoundedBufferChannelReader<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Safety: We're not moving the struct; we're only accessing its fields.
        let self_mut = self.get_mut();

        // Pin the receiver since `poll_recv` requires a `Pin<&mut Receiver<T>>`.
        let mut receiver = Pin::new(&mut self_mut.receiver);
        receiver.poll_recv(cx)
    }
}

/// |--------------------------------------------------------------------------|
/// |                      UnboundedBufferChannel                              |
/// |--------------------------------------------------------------------------|
/// An unbounded buffer channel implementation that wraps Tokio's unbounded MPSC channel.
///
/// This struct provides a message-passing channel with an unlimited buffer size,
/// allowing communication between different parts of async code without backpressure.
///
/// # Type Parameters
///
/// * `T` - The type of values being sent through the channel
///
/// # Fields
///
/// * `sender` - Optional sender half of the channel
/// * `receiver` - Optional receiver half of the channel
/// * `completion` - Tracks the completion state of the stream
///
/// # Examples
///
/// ```
/// use kedo_std::UnboundedBufferChannel;
///
/// let channel: UnboundedBufferChannel<i32> = UnboundedBufferChannel::new();
/// ```
pub struct UnboundedBufferChannel<T> {
    sender: Option<UnboundedBufferChannelWriter<T>>,
    receiver: Option<UnboundedBufferChannelReader<T>>,
    completion: StreamCompletion,
}

impl<T> Drop for UnboundedBufferChannel<T> {
    fn drop(&mut self) {
        self.close();
    }
}

impl<T> UnboundedBufferChannel<T> {
    /// Creates a new unbounded buffer channel.
    ///
    /// An unbounded buffer channel is a channel that can hold an unlimited number of items
    /// without blocking on writes. This implementation uses tokio's unbounded mpsc channel internally.
    ///
    /// # Returns
    ///
    /// Returns a new `UnboundedBufferChannel<T>` instance.
    ///
    /// # Examples
    ///
    /// ```
    /// let channel = UnboundedBufferChannel::<i32>::new(); // Creates an unbounded channel
    /// ```
    pub fn new() -> Self {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

        Self {
            sender: Some(UnboundedBufferChannelWriter { sender }),
            receiver: Some(UnboundedBufferChannelReader { receiver }),
            completion: StreamCompletion::default(),
        }
    }
}

impl<T> BufferChannel<UnboundedBufferChannelReader<T>, UnboundedBufferChannelWriter<T>>
    for UnboundedBufferChannel<T>
{
    /// Acquires a reader for this channel.
    ///
    /// This method takes ownership of the receiver half of the channel and returns
    /// a reader that can be used to receive values. Once the reader is acquired,
    /// no further readers can be acquired and attempts to read directly from the
    /// channel will fail.
    ///
    /// # Returns
    ///
    /// Returns `Some(UnboundedBufferChannelReader<T>)` if the reader was successfully
    /// acquired, or `None` if the reader was already taken.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut channel = UnboundedBufferChannel::<i32>::new();
    /// let reader = channel.acquire_reader().unwrap();
    /// ```
    fn acquire_reader(&mut self) -> Option<UnboundedBufferChannelReader<T>> {
        self.receiver.take()
    }

    /// Acquires a writer for this channel.
    /// This method takes ownership of the sender half of the channel and returns
    /// a writer that can be used to send values. Once the writer is acquired,
    /// no further writers can be acquired and attempts to write directly to the
    /// channel will fail.
    ///
    /// # Returns
    ///
    /// Returns `Some(UnboundedBufferChannelWriter<T>)` if the writer was successfully
    /// acquired, or `None` if the writer was already taken.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut channel = UnboundedBufferChannel::<i32>::new();
    /// let writer = channel.acquire_writer().unwrap();
    /// ```
    fn acquire_writer(&mut self) -> Option<UnboundedBufferChannelWriter<T>> {
        // lets make a copy of the sender
        let sender = self.sender.as_ref()?;
        let sender = sender.sender.clone();
        Some(UnboundedBufferChannelWriter { sender })
    }

    /// Creates a writer for this channel.
    ///
    /// This method creates a writer that can be used to send values to the channel.
    /// Multiple writers can be created from a single channel.
    ///
    /// # Returns
    ///
    /// Returns `Some(UnboundedBufferChannelWriter<T>)` if the writer was successfully
    /// created, or `None` if the sender half has been closed.
    ///
    /// # Examples
    ///
    /// ```
    /// let channel = UnboundedBufferChannel::<i32>::new();
    /// let writer = channel.writer().unwrap();
    /// ```
    fn writer(&self) -> Option<&UnboundedBufferChannelWriter<T>> {
        self.sender.as_ref()
    }

    /// Closes the channel.
    ///
    /// This method closes the sender half of the channel, which means that no more
    /// values can be sent through it. Any in-flight values will still be received.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut channel = UnboundedBufferChannel::<i32>::new();
    /// channel.close();
    /// ```
    fn close(&mut self) {
        let _ = self.sender.take();
        self.completion.close();
    }

    /// Gets the completion state for this channel.
    ///
    /// The completion state can be awaited to be notified when the channel is closed.
    ///
    /// # Returns
    ///
    /// Returns a `StreamCompletion` that can be used to track the channel's completion state.
    ///
    /// # Examples
    ///
    /// ```
    /// let channel = UnboundedBufferChannel::<i32>::new();
    /// let completion = channel.completion();
    /// tokio::spawn(async move {
    ///     completion.await.unwrap();
    ///     println!("Channel was closed");
    /// });
    /// ```
    fn completion(&self) -> StreamCompletion {
        self.completion.clone()
    }
}

impl<T> Default for UnboundedBufferChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// A writer handle for the unbounded buffer channel.
///
/// This struct provides a way to write values to the channel from multiple locations.
///
/// # Type Parameters
///
/// * `T` - The type of values being sent through the channel
#[derive(Debug)]
pub struct UnboundedBufferChannelWriter<T> {
    sender: tokio::sync::mpsc::UnboundedSender<T>,
}

impl<T> Clone for UnboundedBufferChannelWriter<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<T> BufferChannelWriter<T> for UnboundedBufferChannelWriter<T> {
    /// Attempts to write an item to the channel.
    ///
    /// This method is identical to `write` for unbounded channels but is provided
    /// for API consistency with BoundedBufferChannelWriter.
    ///
    /// # Arguments
    ///
    /// * `item` - The item to send through the channel
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the item was successfully sent, or `StreamError::Closed`
    /// if the channel is closed.
    ///
    /// # Examples
    ///
    /// ```
    /// let channel = UnboundedBufferChannel::<i32>::new();
    /// let writer = channel.writer().unwrap();
    /// match writer.try_write(42) {
    ///     Ok(()) => println!("Successfully wrote to channel"),
    ///     Err(e) => println!("Failed to write: {}", e),
    /// }
    /// ```
    fn try_write(&self, item: T) -> Result<(), StreamError> {
        self.sender.send(item).map_err(|_| StreamError::Closed)
    }

    async fn write(&self, _: T) -> Result<(), StreamError> {
        unreachable!("UnboundedBufferChannelWriter does not support async write")
    }
}

/// A reader handle for the unbounded buffer channel.
///
/// This struct provides methods to read values from the channel and can be used
/// as a Stream implementation for working with async iterators.
///
/// # Type Parameters
///
/// * `T` - The type of values being received from the channel
#[derive(Debug)]
pub struct UnboundedBufferChannelReader<T> {
    receiver: tokio::sync::mpsc::UnboundedReceiver<T>,
}

impl<T> BufferChannelReader<T> for UnboundedBufferChannelReader<T> {
    /// Attempts to read an item from the channel without blocking.
    ///
    /// This method will immediately return an error if the channel is empty
    /// or closed, rather than waiting for a value.
    ///
    /// # Returns
    ///
    /// Returns `Ok(T)` with the received item if successful, or a `StreamError` if:
    /// - The channel is empty (`StreamError::Empty`)
    /// - The channel is closed (`StreamError::Closed`)
    ///
    /// # Examples
    ///
    /// ```
    /// let mut channel = UnboundedBufferChannel::<i32>::new();
    /// let mut reader = channel.aquire_reader().unwrap();
    /// match reader.try_read() {
    ///     Ok(value) => println!("Read value: {}", value),
    ///     Err(e) => println!("Error reading: {}", e),
    /// }
    /// ```
    fn try_read(&mut self) -> Result<T, StreamError> {
        self.receiver.try_recv().map_err(|e| match e {
            tokio::sync::mpsc::error::TryRecvError::Empty => StreamError::Empty,
            tokio::sync::mpsc::error::TryRecvError::Disconnected => StreamError::Closed,
        })
    }

    /// Asynchronously reads an item from the channel.
    ///
    /// This method will wait for an item to become available if the channel
    /// is empty. If the channel is closed and empty, it returns an error.
    ///
    /// # Returns
    ///
    /// Returns `Ok(T)` with the received item if successful, or `StreamError::Closed`
    /// if the channel is closed and no more items are available.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut channel = UnboundedBufferChannel::<i32>::new();
    /// let mut reader = channel.aquire_reader().unwrap();
    /// match reader.read().await {
    ///     Ok(value) => println!("Read value: {}", value),
    ///     Err(e) => println!("Error reading: {}", e),
    /// }
    /// ```
    async fn read(&mut self) -> Result<T, StreamError> {
        self.receiver.recv().await.ok_or(StreamError::Closed)
    }
}

impl<T> Stream for UnboundedBufferChannelReader<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Safety: We're not moving the struct; we're only accessing its fields.
        let self_mut = self.get_mut();

        // Pin the receiver since `poll_recv` requires a `Pin<&mut UnboundedReceiver<T>>`.
        let mut receiver = Pin::new(&mut self_mut.receiver);
        receiver.poll_recv(cx)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_bounded_buffer_channel() {
        let mut stream = BoundedBufferChannel::<Vec<u8>>::new(5);
        let mut stream_reader = stream.acquire_reader().unwrap();
        let stream_writer = stream.writer().unwrap();
        for i in 0..5 {
            stream_writer.try_write(vec![i as u8]).unwrap();
        }

        assert_eq!(stream_reader.try_read().unwrap(), vec![0]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![1]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![2]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![3]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_bounded_buffer_channel_async() {
        let mut stream = BoundedBufferChannel::<Vec<u8>>::new(5);
        let mut stream_reader = stream.acquire_reader().unwrap();
        let stream_writer = stream.writer().unwrap();
        for i in 0..5 {
            stream_writer.write(vec![i as u8]).await.unwrap();
        }

        assert_eq!(stream_reader.read().await.unwrap(), vec![0]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![1]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![2]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![3]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_bounded_buffer_channel_async_close() {
        let mut stream = BoundedBufferChannel::<Vec<u8>>::new(5);
        let stream_writer = stream.writer().unwrap();
        for i in 0..5 {
            stream_writer.write(vec![i as u8]).await.unwrap();
        }

        let mut stream_reader = stream.acquire_reader().unwrap();
        stream.close();
        let result = stream_reader.read().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0]);

        drop(stream_reader);
        let stream_writer = stream.writer().unwrap();
        let result = stream_writer.write(vec![5]).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StreamError::Closed);
    }

    #[tokio::test]
    async fn test_internal_stream_resource_completion() {
        let mut stream = BoundedBufferChannel::<Vec<u8>>::new(5);
        let stream_writer = stream.writer().unwrap();
        for i in 0..5 {
            stream_writer.try_write(vec![i as u8]).unwrap();
        }

        let completion = stream.completion();
        let future = async {
            completion.await.unwrap();
        };

        let close_future = async {
            // wait for 2 seconds before closing the stream
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            stream.close();
        };

        tokio::join!(
            tokio::time::timeout(std::time::Duration::from_secs(2), future),
            close_future
        )
        .0
        .unwrap();
    }

    #[test]
    fn test_bounded_buffer_channel_reader() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        let channel_writer = channel.writer().unwrap();
        for i in 0..5 {
            channel_writer.try_write(vec![i as u8]).unwrap();
        }

        let mut reader = channel.acquire_reader().unwrap();
        assert_eq!(reader.try_read().unwrap(), vec![0]);
        assert_eq!(reader.try_read().unwrap(), vec![1]);
        assert_eq!(reader.try_read().unwrap(), vec![2]);
        assert_eq!(reader.try_read().unwrap(), vec![3]);
        assert_eq!(reader.try_read().unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_bounded_buffer_channel_reader_async() {
        let mut channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        let channel_writer = channel.writer().unwrap();
        for i in 0..5 {
            channel_writer.write(vec![i as u8]).await.unwrap();
        }

        let mut reader = channel.acquire_reader().unwrap();
        assert_eq!(reader.read().await.unwrap(), vec![0]);
        assert_eq!(reader.read().await.unwrap(), vec![1]);
        assert_eq!(reader.read().await.unwrap(), vec![2]);
        assert_eq!(reader.read().await.unwrap(), vec![3]);
        assert_eq!(reader.read().await.unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_bound_buffer_channel_limit() {
        let channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        let channel_writer = channel.writer().unwrap();
        for i in 0..5 {
            channel_writer.try_write(vec![i as u8]).unwrap();
        }

        let future = async {
            channel_writer.write(vec![5]).await.unwrap();
        };

        let result =
            tokio::time::timeout(std::time::Duration::from_secs(1), future).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_sync_bound_buffer_channel_limit() {
        let channel = BoundedBufferChannel::<Vec<u8>>::new(5);
        let channel_writer = channel.writer().unwrap();
        for i in 0..5 {
            channel_writer.try_write(vec![i as u8]).unwrap();
        }

        let result = channel_writer.try_write(vec![5]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StreamError::ChannelFull);
    }

    #[test]
    fn test_unbounded_buffer_channel() {
        let mut stream = UnboundedBufferChannel::<Vec<u8>>::new();
        let mut stream_reader = stream.acquire_reader().unwrap();
        let stream_writer = stream.writer().unwrap();
        for i in 0..5 {
            stream_writer.try_write(vec![i as u8]).unwrap();
        }

        assert_eq!(stream_reader.try_read().unwrap(), vec![0]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![1]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![2]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![3]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![4]);
        assert!(stream_reader.try_read().is_err());
    }

    #[tokio::test]
    async fn test_unbounded_buffer_channel_async() {
        let mut stream = UnboundedBufferChannel::<Vec<u8>>::new();
        let mut stream_reader = stream.acquire_reader().unwrap();
        let stream_writer = stream.writer().unwrap();
        for i in 0..5 {
            stream_writer.try_write(vec![i as u8]).unwrap();
        }

        assert_eq!(stream_reader.read().await.unwrap(), vec![0]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![1]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![2]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![3]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_unbounded_buffer_channel_async_close() {
        let mut stream = UnboundedBufferChannel::<Vec<u8>>::new();
        let stream_writer = stream.writer().unwrap();
        for i in 0..5 {
            stream_writer.try_write(vec![i as u8]).unwrap();
        }

        let mut stream_reader = stream.acquire_reader().unwrap();
        stream.close();
        let result = stream_reader.read().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![0]);

        drop(stream_reader);
        let stream_writer = stream.writer().unwrap();
        let result = stream_writer.try_write(vec![5]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StreamError::Closed);
    }

    #[tokio::test]
    async fn test_unbounded_buffer_channel_completion() {
        let mut stream = UnboundedBufferChannel::<Vec<u8>>::new();
        let stream_writer = stream.writer().unwrap();
        for i in 0..5 {
            stream_writer.try_write(vec![i as u8]).unwrap();
        }

        let completion = stream.completion();
        let future = async {
            completion.await.unwrap();
        };

        let close_future = async {
            // wait for 1 second before closing the stream
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            stream.close();
        };

        tokio::join!(
            tokio::time::timeout(std::time::Duration::from_secs(2), future),
            close_future
        )
        .0
        .unwrap();
    }

    #[test]
    fn test_unbounded_buffer_channel_reader() {
        let mut channel = UnboundedBufferChannel::<Vec<u8>>::new();
        let mut stream_reader = channel.acquire_reader().unwrap();
        let stream_writer = channel.writer().unwrap();
        for i in 0..5 {
            stream_writer.try_write(vec![i as u8]).unwrap();
        }

        assert_eq!(stream_reader.try_read().unwrap(), vec![0]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![1]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![2]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![3]);
        assert_eq!(stream_reader.try_read().unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_unbounded_buffer_channel_reader_async() {
        let mut channel = UnboundedBufferChannel::<Vec<u8>>::new();
        let mut stream_reader = channel.acquire_reader().unwrap();
        let stream_writer = channel.writer().unwrap();
        for i in 0..5 {
            stream_writer.try_write(vec![i as u8]).unwrap();
        }

        assert_eq!(stream_reader.read().await.unwrap(), vec![0]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![1]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![2]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![3]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![4]);
    }

    #[tokio::test]
    async fn test_unbounded_channel_large_capacity() {
        let mut channel = UnboundedBufferChannel::<Vec<u8>>::new();
        let mut stream_reader = channel.acquire_reader().unwrap();
        let stream_writer = channel.writer().unwrap();
        // Try writing a large number of items (which would block on bounded channels)
        for i in 0..1000 {
            stream_writer.try_write(vec![i as u8]).unwrap();
        }

        // Read just a few to verify
        assert_eq!(stream_reader.read().await.unwrap(), vec![0]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![1]);
        assert_eq!(stream_reader.read().await.unwrap(), vec![2]);
    }

    #[test]
    fn test_unbounded_buffer_writer() {
        let mut channel = UnboundedBufferChannel::<Vec<u8>>::new();
        let writer = channel.writer().unwrap();

        for i in 0..5 {
            writer.try_write(vec![i as u8]).unwrap();
        }

        let mut reader = channel.acquire_reader().unwrap();
        assert_eq!(reader.try_read().unwrap(), vec![0]);
        assert_eq!(reader.try_read().unwrap(), vec![1]);
        assert_eq!(reader.try_read().unwrap(), vec![2]);
    }
}
