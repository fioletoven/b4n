/// Background observer connection state.
#[derive(Clone, Copy, PartialEq)]
pub enum BgObserverState {
    Idle,
    Connecting,
    Reconnecting,
    Connected,
    Ready,
}

impl From<u8> for BgObserverState {
    fn from(value: u8) -> Self {
        match value {
            0 => BgObserverState::Idle,
            1 => BgObserverState::Connecting,
            2 => BgObserverState::Reconnecting,
            3 => BgObserverState::Connected,
            4 => BgObserverState::Ready,
            _ => BgObserverState::Idle,
        }
    }
}

impl From<BgObserverState> for u8 {
    fn from(value: BgObserverState) -> Self {
        match value {
            BgObserverState::Idle => 0,
            BgObserverState::Connecting => 1,
            BgObserverState::Reconnecting => 2,
            BgObserverState::Connected => 3,
            BgObserverState::Ready => 4,
        }
    }
}
