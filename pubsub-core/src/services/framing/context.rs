use std::task::{Context as TaskContext, Poll};

use libp2p_pubsub_common::service::{BufferedContext, ServiceContext};

use super::events::{ServiceIn, ServiceOut};
use super::service_downstream::DownstreamFramingService;
use super::service_upstream::UpstreamFramingService;

/// A multiplexing service context for the framing upstream and downstream services.
///
/// The service context is a wrapper around the upstream and downstream framing services that
/// multiplexes the input and output events of the two framing services.
///
/// See the [`FramingServiceContext::poll`] method for more details on the event processing
/// strategy.
#[derive(Default)]
pub struct FramingServiceContext {
    downstream: BufferedContext<DownstreamFramingService>,
    upstream: BufferedContext<UpstreamFramingService>,
}

impl ServiceContext for FramingServiceContext {
    type InEvent = ServiceIn;
    type OutEvent = ServiceOut;

    /// Sends a message unconditionally, ignoring any potential errors.
    ///
    /// The event will be enqueued into the corresponding service input mailbox and will be
    /// processed by the service on the next [`FramingServiceContext::poll`] call. See [`poll`](#method.poll) for
    /// more details.
    fn do_send(&mut self, ev: ServiceIn) {
        match ev {
            ServiceIn::Upstream(ev) => self.upstream.do_send(ev),
            ServiceIn::Downstream(ev) => self.downstream.do_send(ev),
        }
    }

    /// Poll the service for events.
    ///
    /// Downstream events are prioritized over upstream events, i.e., if both the upstream
    /// and the downstream service have events to process, the downstream service will be polled
    /// first and its events will be emitted before the upstream service events.
    fn poll(&mut self, cx: &mut TaskContext<'_>) -> Poll<ServiceOut> {
        // Poll the downstream service for events.
        if let Poll::Ready(event) = self.downstream.poll(cx) {
            return Poll::Ready(ServiceOut::Downstream(event));
        }

        // Poll the upstream service for events.
        if let Poll::Ready(event) = self.upstream.poll(cx) {
            return Poll::Ready(ServiceOut::Upstream(event));
        }

        Poll::Pending
    }
}
