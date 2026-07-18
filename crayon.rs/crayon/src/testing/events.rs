//! `ControllerEvent` capture for input tests.
//! Feed synthetic input through a capturing `EventSender` (see
//! `EventSender::capturing`) and drain what came out the other end.

use std::sync::mpsc::Receiver;

use crate::events::ControllerEvent;

/// Everything currently buffered in a capturing receiver, without blocking.
pub fn drain(receiver: &Receiver<ControllerEvent>) -> Vec<ControllerEvent> {
    receiver.try_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_sender::EventSender;

    #[test]
    fn capturing_sender_round_trip() {
        let (sender, receiver) = EventSender::capturing();
        sender.send(ControllerEvent::StrokeStart);
        sender.send(ControllerEvent::StrokeEnd);

        let events = drain(&receiver);
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], ControllerEvent::StrokeStart));
        assert!(matches!(events[1], ControllerEvent::StrokeEnd));
        assert!(drain(&receiver).is_empty(), "drain consumes");
    }
}
