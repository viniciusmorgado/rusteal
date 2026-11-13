use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use rusteal_ffi::RustealErrorCode;

use crate::delegate_registry::{register_callback, unregister_callback};
use crate::error::{RustealError, RustealResult, check_ffi};
use crate::ffi_dispatch;
use crate::lock_or_recover;
use crate::sync::Mutex;

/// An HTTP request sent through the engine's HTTP module: build it, then await `send()`.
#[derive(Debug, Clone)]
#[must_use = "a request does nothing until it is sent"]
pub struct Request {
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    timeout: Option<Duration>,
}

/// The response to a [`Request`].
#[derive(Debug, Clone)]
pub struct Response {
    pub status: i32,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    /// The value of the header `name`, matched without case.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// The body as UTF-8 text, with invalid bytes replaced.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

pub fn request(method: impl Into<String>, url: impl Into<String>) -> Request {
    Request {
        method: method.into(),
        url: url.into(),
        headers: Vec::new(),
        body: Vec::new(),
        timeout: None,
    }
}

pub fn get(url: impl Into<String>) -> Request {
    request("GET", url)
}

pub fn post(url: impl Into<String>) -> Request {
    request("POST", url)
}

impl Request {
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sends the request and resumes with the response; awaited in a task started with
    /// `task::spawn_local`. A request the server answered with an error status is an `Ok`
    /// response; a request that got no answer (no connection, a timeout) is an `Err`.
    pub async fn send(self) -> RustealResult<Response> {
        let headers: String = self
            .headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}"))
            .collect::<Vec<_>>()
            .join("\n");

        let mut call = Call::new();

        check_ffi(unsafe {
            ffi_dispatch::world_http_send(
                self.method.as_ptr(),
                self.method.len() as u32,
                self.url.as_ptr(),
                self.url.len() as u32,
                headers.as_ptr(),
                headers.len() as u32,
                self.body.as_ptr(),
                self.body.len() as u32,
                self.timeout.map_or(0.0, |t| t.as_secs_f32()),
                call.id,
            )
        })?;

        (&mut call).await;

        let response = call.finish()?;

        match response {
            Some(response) => Ok(response),
            None => Err(RustealError::invalid_operation(format!(
                "http: {} {} got no response",
                self.method, self.url
            ))),
        }
    }
}

#[derive(Default)]
struct State {
    done: bool,
    waker: Option<Waker>,
}

struct Finish(Arc<Mutex<State>>);

impl Finish {
    fn finish(&self) {
        let waker = {
            let mut state = lock_or_recover(&self.0);
            state.done = true;
            state.waker.take()
        };

        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl Drop for Finish {
    fn drop(&mut self) {
        self.finish();
    }
}

struct Call {
    id: u64,
    state: Arc<Mutex<State>>,
    finished: bool,
}

impl Call {
    fn new() -> Self {
        let state = Arc::new(Mutex::new(State::default()));
        let finish = Finish(state.clone());
        let id = register_callback(move |_| finish.finish());

        Call {
            id,
            state,
            finished: false,
        }
    }

    fn finish(&mut self) -> RustealResult<Option<Response>> {
        self.finished = true;
        unregister_callback(self.id);

        let mut headers = vec![0u8; 4096];
        let mut body = vec![0u8; 64 * 1024];

        loop {
            let mut status = 0i32;
            let mut headers_len = 0u32;
            let mut body_len = 0u32;
            let mut succeeded = false;

            let code = unsafe {
                ffi_dispatch::world_http_finish(
                    self.id,
                    &mut status,
                    headers.as_mut_ptr(),
                    headers.len() as u32,
                    &mut headers_len,
                    body.as_mut_ptr(),
                    body.len() as u32,
                    &mut body_len,
                    &mut succeeded,
                )
            };

            if code == RustealErrorCode::BufferTooSmall {
                headers.resize((headers_len as usize).max(headers.len()), 0);
                body.resize((body_len as usize).max(body.len()), 0);
                continue;
            }

            check_ffi(code)?;

            if !succeeded {
                return Ok(None);
            }

            headers.truncate(headers_len as usize);
            body.truncate(body_len as usize);

            let headers = String::from_utf8_lossy(&headers)
                .lines()
                .filter_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    Some((name.trim().to_string(), value.trim().to_string()))
                })
                .collect();

            return Ok(Some(Response {
                status,
                headers,
                body,
            }));
        }
    }
}

impl Future for Call {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = lock_or_recover(&self.state);

        if state.done {
            return Poll::Ready(());
        }

        state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl Drop for Call {
    fn drop(&mut self) {
        if self.finished {
            return;
        }

        if crate::api::is_api_initialized() {
            let _ = unsafe { ffi_dispatch::world_http_cancel(self.id) };
        }

        unregister_callback(self.id);
    }
}

#[doc(hidden)]
pub fn requests_in_progress() -> u32 {
    unsafe { ffi_dispatch::world_http_in_progress() }
}
