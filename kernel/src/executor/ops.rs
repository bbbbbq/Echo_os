use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use log::info;
pub struct Yield(bool);

impl Yield {
    pub const fn new() -> Self {
        Self(false)
    }
}

impl Future for Yield {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.0 {
            true => Poll::Ready(()),
            false => {
                self.0 = true;
                Poll::Pending
            }
        }
    }
}

pub async fn yield_now() {
    info!("Yielding to other tasks");
    Yield::new().await;
}
