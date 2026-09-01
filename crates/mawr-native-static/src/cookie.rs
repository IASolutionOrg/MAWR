use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use reqwest::cookie::{CookieStore, Jar};
use reqwest::header::HeaderValue;

#[derive(Debug)]
pub(crate) struct BoundedCookieJar {
    inner: Jar,
    accepted_bytes: AtomicU64,
    max_bytes: u64,
    exceeded: AtomicBool,
}

impl BoundedCookieJar {
    pub(crate) fn new(max_bytes: u64) -> Self {
        Self {
            inner: Jar::default(),
            accepted_bytes: AtomicU64::new(0),
            max_bytes,
            exceeded: AtomicBool::new(false),
        }
    }

    pub(crate) fn is_exceeded(&self) -> bool {
        self.exceeded.load(Ordering::Acquire)
    }
}

impl CookieStore for BoundedCookieJar {
    fn set_cookies(
        &self,
        cookie_headers: &mut dyn Iterator<Item = &HeaderValue>,
        url: &reqwest::Url,
    ) {
        for header in cookie_headers {
            let length = header.as_bytes().len() as u64;
            let accepted = self
                .accepted_bytes
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                    current
                        .checked_add(length)
                        .filter(|next| *next <= self.max_bytes)
                })
                .is_ok();
            if accepted {
                self.inner.set_cookies(&mut std::iter::once(header), url);
            } else {
                self.exceeded.store(true, Ordering::Release);
            }
        }
    }

    fn cookies(&self, url: &reqwest::Url) -> Option<HeaderValue> {
        self.inner.cookies(url)
    }
}
