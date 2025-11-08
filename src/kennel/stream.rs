use std::task::{Context, Poll};

use pin_project_lite::pin_project;
use rocket::futures::stream::{Fuse, Stream, StreamExt};

pin_project! {
    #[derive(Debug)]
    pub struct GreedyZip<S1: Stream, S2: Stream> {
        #[pin]
        stream1: Fuse<S1>,
        #[pin]
        stream2: Fuse<S2>,
    }
}

impl<S1: Stream, S2: Stream> GreedyZip<S1, S2> {
    fn new(stream1: S1, stream2: S2) -> Self {
        Self {
            stream1: stream1.fuse(),
            stream2: stream2.fuse(),
        }
    }

    pub fn get_mut(&mut self) -> (&mut S1, &mut S2) {
        (self.stream1.get_mut(), self.stream2.get_mut())
    }
}

impl<S1, S2> Stream for GreedyZip<S1, S2>
where
    S1: Stream,
    S2: Stream,
{
    type Item = (Option<S1::Item>, Option<S2::Item>);

    fn poll_next(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        match (
            this.stream1.as_mut().poll_next(cx),
            this.stream2.as_mut().poll_next(cx),
        ) {
            (Poll::Ready(Some(item1)), Poll::Ready(Some(item2))) => {
                Poll::Ready(Some((Some(item1), Some(item2))))
            }
            (Poll::Ready(Some(item1)), _) => Poll::Ready(Some((Some(item1), None))),
            (_, Poll::Ready(Some(item2))) => Poll::Ready(Some((None, Some(item2)))),
            (_, _) if this.stream1.is_done() && this.stream2.is_done() => Poll::Ready(None),
            (_, _) => Poll::Pending,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower1, upper1) = self.stream1.size_hint();
        let (lower2, upper2) = self.stream2.size_hint();

        let lower = usize::min(lower1, lower2);
        let upper = match (upper1, upper2) {
            (Some(u1), Some(u2)) => usize::checked_add(u1, u2),
            (Some(u1), None) => Some(u1),
            (None, Some(u2)) => Some(u2),
            (None, None) => None,
        };

        (lower, upper)
    }
}

pub fn greedy_zip<S1: Stream, S2: Stream>(stream1: S1, stream2: S2) -> GreedyZip<S1, S2> {
    GreedyZip::new(stream1, stream2)
}
